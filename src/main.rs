#![allow(proc_macro_derive_resolution_fallback)] // See: https://github.com/diesel-rs/diesel/issues/1785

extern crate actix;
extern crate actix_web;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
extern crate authy;
extern crate chrono;
extern crate dotenv;
extern crate env_logger;
extern crate failure;
extern crate failure_derive;
#[macro_use]
extern crate log;

mod db;
mod error;
mod models;
mod schema;

use self::db::{
    CreateBeer, CreateBrewery, CreateDrink, DatabaseExecutor, GetBeerByName, GetBreweryByName,
    GetDrinks, LookupIdentiy,
};

use std::str::FromStr;

use actix::prelude::*;
use actix_web::middleware::{cors, Logger};
use actix_web::*;
use actix_web::{server, App, HttpRequest, Responder};
use chrono::naive::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use futures::future::Either;
use futures::Future;
use std::convert::From;

struct AppState {
    db: Addr<db::DatabaseExecutor>,
}

fn index(_: &HttpRequest<AppState>) -> impl Responder {
    "Hello World".to_owned()
}

fn get_drinks(state: State<AppState>) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(GetDrinks)
        .from_err()
        .and_then(|res| match res {
            Ok(drinks) => Ok(HttpResponse::Ok().json(drinks)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

#[derive(Deserialize)]
struct DrinkForm {
    /// Date on which the drink was had.
    drank_on: NaiveDate,

    /// The name of the beer.
    beer: String,

    /// The name of the beer's brewery.
    brewery: String,

    /// Rating of the beer.
    rating: i16,

    /// A comment/opinion about the beer.
    comment: Option<String>,
}

/// Route handler for creating new drink records
///
/// Expects the following POST data:
///
/// - `drank_on`: The date on which the drink was had (yyyy-mm-dd).
/// - `beer`: The name of the beer
/// - `brewery`: The name of the brewery
/// - `rating`: The rating of the beer, 0 - 5
/// - `comment`: An optional comment about the beer
///
/// If no records correspond to the `beer` or `brewery` names, new records will be created.
fn new_drink((details, state): (Form<DrinkForm>, State<AppState>)) -> FutureResponse<HttpResponse> {
    type DbAddr = Addr<DatabaseExecutor>;

    // Save these for later
    let beer_name = details.beer.clone();
    let db_addr_copy1 = state.db.clone();
    let db_addr_copy2 = state.db.clone();
    let db_addr_copy3 = state.db.clone();

    /*********************************************/
    /*  Closures for database operations         */
    /*********************************************/

    // This closure will create a new brewery record with the given `name`.
    let create_brewery = |db_addr: DbAddr, name: String| {
        db_addr
            .send(CreateBrewery { name: name })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will create a new beer record, given a `name` and its `brewery_id`.
    let create_beer = |db_addr: DbAddr, name: String, brewery_id: i32| {
        db_addr
            .send(CreateBeer { name, brewery_id })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will lookup a brewery given its `name` and,
    // if no matching record is found, will insert a new one.
    let get_brewery = |db_addr: DbAddr, name: String| {
        db_addr
            .send(GetBreweryByName { name: name.clone() })
            .from_err::<Error>()
            .map(move |res| match res {
                Ok(Some(brewery)) => Either::A(futures::future::result(Ok(brewery))),
                Ok(None) => Either::B(create_brewery(db_addr, name)),
                Err(e) => Either::A(futures::future::result(Err(actix_web::Error::from(e)))),
            })
            .from_err::<actix_web::Error>()
            .flatten()
    };

    // This closure will lookup a beer given its `name` and `brewery_id` and,
    // will insert a new one if no record is found.
    let get_beer = move |db_addr: DbAddr, name: String, brewery_id: i32| {
        db_addr
            .send(GetBeerByName {
                name: name.clone(),
                brewery_id: brewery_id,
            })
            .from_err()
            .and_then(move |res| match res {
                Ok(Some(beer)) => Either::A(futures::future::result(Ok(beer))),
                Ok(None) => Either::B(create_beer(db_addr, name, brewery_id)),
                Err(e) => Either::A(futures::future::result(Err(actix_web::Error::from(e)))),
            })
    };

    // This will insert a new Drink record
    let record_drink = |db_addr: DbAddr, drink: CreateDrink| {
        db_addr
            .send(drink)
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    /*********************************************/
    /* Begin actual function execution           */
    /*********************************************/

    // Look up the given brewery, and create a new record if one is not found
    get_brewery(db_addr_copy1, details.brewery.clone())
        // Then lookup the beer by name, and create a new record if it is not found.
        .and_then(move |brewery| get_beer(db_addr_copy2, beer_name, brewery.id))
        // Finally, insert a record of the individual drink
        .and_then(move |beer| {
            let drink = CreateDrink {
                drank_on: details.drank_on,
                beer_id: beer.id,
                rating: details.rating,
                comment: details.comment.clone(),
            };

            record_drink(db_addr_copy3, drink)
        })
        // Format the result for output
        .then(|res| match res {
            Ok(drink) => Ok(HttpResponse::Ok().json(drink)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

#[derive(Deserialize)]
struct AuthForm {
    identity: String,
}

fn begin_auth((form, state): (Form<AuthForm>, State<AppState>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(LookupIdentiy {
            identifier: form.identity.clone(),
        })
        .from_err()
        .and_then(|res| match res {
            Ok(ident) => Ok(HttpResponse::Ok().json(ident)),
            Err(e) => {
                error!("{}", e);
                Ok(HttpResponse::InternalServerError().into())
            }
        })
        .responder()
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let sys = actix::System::new("mug-club");

    // Read the port on which to listen.
    let port = u16::from_str(&std::env::var("PORT").unwrap_or("1234".into()))
        .expect("Failed to parse $PORT!");

    // Read the IP address on which to listen
    let ip = std::net::IpAddr::from_str(&std::env::var("LISTEN_IP").unwrap_or("127.0.0.1".into()))
        .expect("Failed to parse $LISTEN_IP");

    // Construct the full Socket address
    let listen_addr = std::net::SocketAddr::new(ip, port);

    // Create a connection pool to the database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set!");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool!");

    // Start 3 database executor actors to handle operations in parallel.
    let addr = SyncArbiter::start(3, move || DatabaseExecutor(pool.clone()));

    server::new(move || {
        App::with_state(AppState { db: addr.clone() })
            .middleware(Logger::default())
            .middleware(cors::Cors::build().finish())
            .resource("/", |r| r.h(index))
            .resource("/drink", |r| {
                r.method(http::Method::GET).with_async(get_drinks);
                r.method(http::Method::POST).with_async(new_drink)
            })
            .resource("/auth", |r| {
                r.method(http::Method::POST).with_async(begin_auth)
            })
    })
    .bind(&listen_addr)
    .unwrap()
    .start();

    info!("Listening on {}", listen_addr);

    let _ = sys.run();
}
