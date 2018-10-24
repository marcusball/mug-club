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
extern crate chrono;
extern crate dotenv;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;

mod db;
mod error;
mod models;
mod schema;

use self::db::{
    CreateBeer, CreateBrewery, CreateDrink, DatabaseExecutor, GetBeerByName, GetBreweryByName,
    GetDrinks,
};

use std::str::FromStr;

use actix::prelude::*;
use actix_web::middleware::{cors, Logger};
use actix_web::*;
use actix_web::{fs, server, App, HttpRequest, Responder};
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
    // This closure will create a new brewery record with the given `name`.
    let db_addr = state.db.clone();
    let create_brewery = move |name: String| {
        db_addr
            .send(CreateBrewery { name: name })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will create a new beer record, given a `name` and its `brewery_id`.
    let db_addr = state.db.clone();
    let create_beer = move |name: String, brewery_id: i32| {
        db_addr
            .send(CreateBeer { name, brewery_id })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // Look up a brewery by name; If one is not found, create a new record.
    //
    // This is a little messy, but for info on the use of `Either`,
    // see: https://github.com/rust-lang-nursery/futures-rs/issues/683
    let brewery_name = details.brewery.clone();
    let get_brewery = state
        .db
        .send(GetBreweryByName {
            name: brewery_name.clone(),
        })
        .from_err::<Error>()
        .map(move |res| match res {
            Ok(Some(brewery)) => Either::A(futures::future::result(Ok(brewery))),
            Ok(None) => Either::B(create_brewery(brewery_name.clone())),
            Err(e) => Either::A(futures::future::result(Err(actix_web::Error::from(e)))),
        })
        .from_err::<actix_web::Error>()
        .flatten();

    // Look up a beer by name; if one is not found, create a new record
    let beer_name = details.beer.clone();
    let db_addr = state.db.clone();
    let get_beer = get_brewery.and_then(move |brewery| {
        db_addr
            .send(GetBeerByName {
                name: beer_name.clone(),
                brewery_id: brewery.id,
            })
            .from_err()
            .and_then(move |res| match res {
                Ok(Some(beer)) => Either::A(futures::future::result(Ok(beer))),
                Ok(None) => Either::B(create_beer(beer_name, brewery.id)),
                Err(e) => Either::A(futures::future::result(Err(actix_web::Error::from(e)))),
            })
    });

    // Create a new drink record
    let db_addr = state.db.clone();
    let drank_on = details.drank_on;
    let rating = details.rating;
    let comment = details.comment.clone();
    get_beer
        .and_then(move |beer| {
            db_addr
                .send(CreateDrink {
                    drank_on: drank_on,
                    beer_id: beer.id,
                    rating: rating,
                    comment: comment,
                })
                .from_err()
                .and_then(|res| match res {
                    Ok(drink) => Ok(HttpResponse::Ok().json(drink)),
                    Err(_) => Ok(HttpResponse::InternalServerError().into()),
                })
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
    })
    .bind(&listen_addr)
    .unwrap()
    .start();

    println!("Listening on {}", listen_addr);

    let _ = sys.run();
}
