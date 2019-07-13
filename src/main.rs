#![allow(proc_macro_derive_resolution_fallback)] // See: https://github.com/diesel-rs/diesel/issues/1785

extern crate actix_web;
extern crate actix_cors;
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

use std::convert::From;
use std::str::FromStr;

use actix_web::middleware::Logger;
use actix_web::*;
use actix_web::{HttpServer, App, HttpRequest, Responder};
use actix_cors::Cors;
use authy::AuthyError;
use chrono::naive::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
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

/*
fn get_drinks((person, state): (models::Person, State<AppState>)) -> FutureResponse<HttpResponse> {
    #[derive(Serialize)]
    #[serde(rename = "drinks")]
    struct Drinks(Vec<ExpandedDrink>);

    state
        .db
        .send(GetDrinks {
            person_id: person.id,
        })
        .from_err()
        .and_then(|res| match res {
            Ok(drinks) => Ok(HttpResponse::Ok().json(ApiResponse::success(Drinks(drinks)))),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}
*/

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

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route("/", web::get().to(index))
            .route("/wakeup", web::get().to(wakeup))
    })
    .bind(&listen_addr)
    .unwrap()
    .run()
    .unwrap();
}
