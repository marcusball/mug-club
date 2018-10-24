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

use std::str::FromStr;

use actix::prelude::*;
use actix_web::middleware::{cors, Logger};
use actix_web::*;
use actix_web::{fs, server, App, HttpRequest, Responder};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use futures::Future;

struct AppState;

fn index(_: &HttpRequest<AppState>) -> impl Responder {
    "Hello World".to_owned()
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

    server::new(move || {
        App::with_state(AppState)
            .middleware(Logger::default())
            .middleware(cors::Cors::build().finish())
            .resource("/", |r| r.h(index))
    })
    .bind(&listen_addr)
    .unwrap()
    .start();

    println!("Listening on {}", listen_addr);

    let _ = sys.run();
}
