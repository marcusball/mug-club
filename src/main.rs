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
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate textnonce;

mod db;
mod error;
mod models;
mod schema;

use self::db::{
    CreateBeer, CreateBrewery, CreateDrink, DatabaseExecutor, GetBeerByName, GetBreweryByName,
    GetDrinks, LookupIdentiy, StartSession,
};

use std::convert::From;
use std::str::FromStr;

use actix::prelude::*;
use actix_web::middleware::{cors, Logger};
use actix_web::*;
use actix_web::{server, App, HttpRequest, Responder};
use authy::AuthyError;
use chrono::naive::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use futures::future::Either;
use futures::Future;
use regex::Regex;

struct AppState {
    db: Addr<db::DatabaseExecutor>,
}

fn index(_: &HttpRequest<AppState>) -> impl Responder {
    "Hello World".to_owned()
}

fn get_drinks((person, state): (models::Person, State<AppState>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(GetDrinks {
            person_id: person.id,
        })
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
    (person, details, state): (models::Person, Form<DrinkForm>, State<AppState>),
) -> FutureResponse<HttpResponse> {
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
                person_id: person.id,
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
    country_code: u16,
    phone_number: String,
    code: Option<String>,
}

fn begin_auth((form, _state): (Form<AuthForm>, State<AppState>)) -> FutureResponse<HttpResponse> {
    use authy::api::phone;

    lazy_static! {
        // See: https://github.com/authy/authy-form-helpers/blob/be2081cd44041ba61173658c100471c8ff7302b9/src/form.authy.js#L693
        static ref RE: Regex =
            Regex::new(r"^([0-9][0-9][0-9])\W*([0-9][0-9]{2})\W*([0-9]{0,5})$").unwrap();
    }

    // Check to make sure that the identity submitted appears to be a phone number
    if !RE.is_match(&form.phone_number) {
        info!(
            "Received invalid phone number '{}' '{}'!",
            form.country_code, form.phone_number
        );
        return futures::future::ok(HttpResponse::BadRequest().body("Invalid phone number"))
            .responder();
    }

    let client = authy::Client::new(
        "https://api.authy.com",
        &std::env::var("AUTHY_API_KEY").expect("An authy API key is required!"),
    );

    let (status, _start) = match phone::start(
        &client,
        phone::ContactType::SMS,
        form.country_code,
        &form.phone_number,
        Some(6),
        None,
    ) {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to start phone number verification! Error: {}", e);
            return futures::future::ok(HttpResponse::InternalServerError().into()).responder();
        }
    };

    futures::future::ok(HttpResponse::Ok().body(status.message)).responder()
}

fn complete_auth((form, state): (Form<AuthForm>, State<AppState>)) -> FutureResponse<HttpResponse> {
    use authy::api::phone;

    type DbAddr = Addr<DatabaseExecutor>;

    /*********************************************/
    /*  Closures for database operations         */
    /*********************************************/

    let lookup_idenity = |db_addr: DbAddr, country_code: u16, phone_number: String| {
        db_addr
            .send(LookupIdentiy {
                identifier: format!("{}{}", country_code, phone_number),
            })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    let start_session = |db_addr: DbAddr, person_id: i32| {
        db_addr
            .send(StartSession { person_id })
            .from_err()
            .and_then(|res| res)
            .map_err(|e| actix_web::Error::from(e))
    };

    /*********************************************/
    /*  Begin request handling logic             */
    /*********************************************/

    lazy_static! {
        // See: https://github.com/authy/authy-form-helpers/blob/be2081cd44041ba61173658c100471c8ff7302b9/src/form.authy.js#L693
        static ref RE: Regex =
            Regex::new(r"^([0-9][0-9][0-9])\W*([0-9][0-9]{2})\W*([0-9]{0,5})$").unwrap();
    }

    // Make sure some kind of verification code was submitted
    if form.code.is_none() {
        info!("Verification code was submitted!");
        return futures::future::ok(HttpResponse::BadRequest().body("Missing verification code!"))
            .responder();
    }

    // Check to make sure that the identity submitted appears to be a phone number
    if !RE.is_match(&form.phone_number) {
        info!(
            "Received invalid phone number '{}' '{}'!",
            form.country_code, form.phone_number
        );
        return futures::future::ok(HttpResponse::BadRequest().body("Invalid phone number"))
            .responder();
    }

    /*********************************************/
    /*  Verify the phone number and code         */
    /*********************************************/

    let client = authy::Client::new(
        "https://api.authy.com",
        &std::env::var("AUTHY_API_KEY").expect("An authy API key is required!"),
    );

    let verification_code = form.code.clone().unwrap_or("wtf".into());

    // Attempt to verify the verification code
    let verification_status = phone::check(
        &client,
        form.country_code,
        &form.phone_number,
        &verification_code,
    );

    match verification_status {
        Ok(status) => {
            // If the verification code was invalid, return an error
            if !status.success {
                warn!(
                    "Invalid verification code, '{}', submitted for '{}' '{}'!",
                    verification_code, form.country_code, form.phone_number
                );
                return futures::future::ok(
                    HttpResponse::Forbidden().body("Invalid verification code"),
                )
                .responder();
            }

            // Verification was correct
            info!(
                "Phone number {} {} verified!",
                form.country_code, form.phone_number
            );
        }
        Err(e) => {
            return match e {
                // If there was an internal error, that the Authy crate has bubbled up.
                AuthyError::RequestError(e)
                | AuthyError::IoError(e)
                | AuthyError::JsonParseError(e) => {
                    // Something awful happened
                    warn!(
                        "Unable to verify code, '{}', submitted for '{}' '{}'! Error: {}",
                        verification_code, form.country_code, form.phone_number, e
                    );

                    futures::future::ok(HttpResponse::InternalServerError().into()).responder()
                }
                // If the verification code was incorrect
                // The Authy crate currently returns this as an Unauthorized API Key error.
                AuthyError::UnauthorizedKey(_) => {
                    warn!(
                        "Invalid verification code, '{}', submitted for '{}' '{}'!",
                        verification_code, form.country_code, form.phone_number
                    );

                    futures::future::ok(HttpResponse::Forbidden().body("Invalid verification code"))
                        .responder()
                }
                // If we received some other Authy error response.
                e => {
                    warn!(
                        "Unexpected authy error during verification, '{}', submitted for '{}' '{}'! Error: {}",
                        verification_code, form.country_code, form.phone_number, e
                    );

                    futures::future::ok(HttpResponse::Forbidden().body("Invalid verification code"))
                        .responder()
                }
            };
        }
    }

    /*********************************************/
    /*  Verified, find identity, start session   */
    /*********************************************/

    let db_addr = state.db.clone();

    lookup_idenity(
        state.db.clone(),
        form.country_code,
        form.phone_number.clone(),
    )
    .and_then(move |ident| start_session(db_addr, ident.person_id))
    .then(move |res| match res {
        Ok(session) => {
            info!(
                "Successfully verified identity for person {}",
                session.person_id
            );

            Ok(HttpResponse::Ok().json(session))
        }
        Err(e) => {
            error!("Failed to start session! Error: {}", e);

            Ok(HttpResponse::InternalServerError().into())
        }
    })
    .responder()
}

fn test_auth(person: models::Person) -> impl Responder {
    #[derive(Serialize)]
    struct TestResponse {
        message: String,
    }

    HttpResponse::Ok().json(TestResponse {
        message: format!("Hello person {}", person.id),
    })
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    // Make sure an authy API key is set before starting.
    let _ = std::env::var("AUTHY_API_KEY").expect("An authy API key is required!");

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
            .resource("/auth/verify", |r| {
                r.method(http::Method::POST).with_async(complete_auth)
            })
            .resource("/auth/test", |r| r.with(test_auth))
    })
    .bind(&listen_addr)
    .unwrap()
    .start();

    info!("Listening on {}", listen_addr);

    let _ = sys.run();
}
