#![allow(proc_macro_derive_resolution_fallback)] // See: https://github.com/diesel-rs/diesel/issues/1785

extern crate actix_cors;
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
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate log;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate textnonce;

mod api;
mod db;
mod error;
mod models;
mod schema;

use self::api::{ApiResponse, ResponseStatus};
use self::db::{
    Connection, CreateBeer, CreateBrewery, CreateDrink, DeleteDrink, ExpandedDrink, GetBeerByName,
    GetBreweryByName, GetDrink, GetDrinks, Pool,
};

use std::convert::From;
use std::str::FromStr;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::*;
use actix_web::{App, HttpRequest, HttpServer, Responder};
use authy::AuthyError;
use chrono::naive::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use futures::future::Either;
use futures::Future;
use regex::Regex;

fn index() -> impl Responder {
    #[derive(Serialize)]
    #[serde(rename = "message")]
    struct TestResponse(String);

    HttpResponse::Ok().json(ApiResponse::success(TestResponse("Hello world!".into())))
}

// Dummy method. Just wanted a route for the front-end to ping to make up the heroku instance.
fn wakeup() -> impl Responder {
    #[derive(Serialize)]
    #[serde(rename = "message")]
    struct TestResponse(String);

    HttpResponse::Ok().json(ApiResponse::success(TestResponse("👍".into())))
}

fn get_drinks(
    pool: web::Data<Pool>,
    person: models::Person,
) -> impl Future<Item = HttpResponse, Error = Error> {
    #[derive(Serialize)]
    #[serde(rename = "drinks")]
    struct Drinks(Vec<ExpandedDrink>);

    db::execute(
        &pool,
        GetDrinks {
            person_id: person.id,
        },
    )
    .from_err()
    .and_then(|res| match res {
        Ok(drinks) => Ok(HttpResponse::Ok().json(ApiResponse::success(Drinks(drinks)))),
        Err(_) => Ok(HttpResponse::InternalServerError().into()),
    })
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
/// Requires a valid session token in the `Authorization` header.
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
fn new_drink(
    pool: web::Data<Pool>,
    person: models::Person,
    details: web::Form<DrinkForm>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    // Save these for later
    let beer_name = details.beer.clone();

    /*********************************************/
    /*  Closures for database operations         */
    /*********************************************/

    // This closure will create a new brewery record with the given `name`.
    let create_brewery = |pool: &Pool, name: String| {
        db::execute(pool, CreateBrewery { name: name })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will create a new beer record, given a `name` and its `brewery_id`.
    let create_beer = |pool: &Pool, name: String, brewery_id: i32| {
        db::execute(pool, CreateBeer { name, brewery_id })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will lookup a brewery given its `name` and,
    // if no matching record is found, will insert a new one.
    let get_brewery = |pool: &Pool, name: String| {
        let pool_clone = pool.clone();
        db::execute(pool, GetBreweryByName { name: name.clone() })
            .from_err::<Error>()
            .map(move |res| match res {
                Ok(Some(brewery)) => Either::A(futures::future::result(Ok(brewery))),
                Ok(None) => Either::B(create_brewery(&pool_clone, name)),
                Err(e) => Either::A(futures::future::result(Err(actix_web::Error::from(e)))),
            })
            .from_err::<actix_web::Error>()
            .flatten()
    };

    // This closure will lookup a beer given its `name` and `brewery_id` and,
    // will insert a new one if no record is found.
    let get_beer = move |pool: &Pool, name: String, brewery_id: i32| {
        let pool_clone = pool.clone();
        db::execute(
            pool,
            GetBeerByName {
                name: name.clone(),
                brewery_id: brewery_id,
            },
        )
        .from_err()
        .and_then(move |res| match res {
            Ok(Some(beer)) => Either::A(futures::future::result(Ok(beer))),
            Ok(None) => Either::B(create_beer(&pool_clone, name, brewery_id)),
            Err(e) => Either::A(futures::future::result(Err(actix_web::Error::from(e)))),
        })
    };

    // This will insert a new Drink record
    let record_drink = |pool: &Pool, drink: CreateDrink| {
        db::execute(pool, drink)
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    // Get an ExpandedDrink record by ID
    let get_drink = |pool: &Pool, drink_id: i32| {
        db::execute(pool, GetDrink { drink_id })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    /*********************************************/
    /* Begin actual function execution           */
    /*********************************************/

    let pool_clone_1 = pool.clone();
    let pool_clone_2 = pool.clone();

    // Look up the given brewery, and create a new record if one is not found
    get_brewery(&pool, details.brewery.clone())
        // Then lookup the beer by name, and create a new record if it is not found.
        .and_then(move |brewery| get_beer(&pool, beer_name, brewery.id))
        // Finally, insert a record of the individual drink
        .and_then(move |beer| {
            let drink = CreateDrink {
                person_id: person.id,
                drank_on: details.drank_on,
                beer_id: beer.id,
                rating: details.rating,
                comment: details.comment.clone(),
            };

            record_drink(&pool_clone_1, drink)
        })
        .and_then(move |drink| get_drink(&pool_clone_2, drink.id))
        // Format the result for output
        .then(|res| match res {
            Ok(drink) => Ok(HttpResponse::Ok().json(ApiResponse::success(drink))),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
}

#[derive(Deserialize)]
struct DrinkIdForm {
    id: i32,
}

fn delete_drink(
    person: models::Person,
    info: web::Path<DrinkIdForm>,
    pool: web::Data<Pool>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    db::execute(
        &pool,
        DeleteDrink {
            drink_id: info.id,
            person_id: person.id,
        },
    )
    .from_err()
    .and_then(move |res| match res {
        Ok(0) => {
            let not_found = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Fail)
                .add_message("Could not find that drink".into());

            Ok(HttpResponse::NotFound().json(not_found))
        }
        Ok(1) => {
            let deleted = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Success)
                .add_message("Deleted".into());

            Ok(HttpResponse::Ok().json(deleted))
        }
        Ok(n) => {
            error!("Person {} somehow deleted {} drinks!", person.id, n);

            let unexpected_error = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Error)
                .add_message("An unexpected error occurred".into());

            Ok(HttpResponse::InternalServerError().json(unexpected_error))
        }
        Err(e) => {
            error!(
                "Unable to delete drink for person {}! Error: {}",
                person.id, e
            );

            let unexpected_error = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Error)
                .add_message("An unexpected error occurred".into());

            Ok(HttpResponse::InternalServerError().json(unexpected_error))
        }
    })
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    // Make sure an authy API key is set before starting.
    let _ = std::env::var("AUTHY_API_KEY").expect("An authy API key is required!");

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
    let pool = Pool::new(manager).expect("Failed to create database connection pool!");

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route("/", web::get().to(index))
            .route("/wakeup", web::get().to(wakeup))
            .service(
                web::scope("/drink").service(
                    web::resource("")
                        .route(web::get().to_async(get_drinks))
                        .route(web::post().to_async(new_drink)),
                )
                .service(
                    web::resource("/{id}")
                        .route(web::delete().to_async(delete_drink))
                ),
            )
    })
    .bind(&listen_addr)
    .unwrap()
    .run()
    .unwrap();
}
