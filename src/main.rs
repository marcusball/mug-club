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

use self::db::{CreateDrink, DatabaseExecutor, GetDrinks};

use std::str::FromStr;

use actix::prelude::*;
use actix_web::middleware::{cors, Logger};
use actix_web::*;
use actix_web::{fs, server, App, HttpRequest, Responder};
use chrono::naive::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use futures::Future;

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
    drank_on: NaiveDate,
    beer_id: i32,
    rating: i16,
    comment: Option<String>,
}

fn new_drink((details, state): (Form<DrinkForm>, State<AppState>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(CreateDrink {
            drank_on: details.drank_on.clone(),
            beer_id: details.beer_id,
            rating: details.rating,
            comment: details.comment.clone(),
        })
        .from_err()
        .and_then(|res| match res {
            Ok(drink) => Ok(HttpResponse::Ok().json(drink)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
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
