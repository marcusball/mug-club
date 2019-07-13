use chrono::naive::NaiveDate;
use chrono::{Duration, Utc};
use diesel;
use diesel::prelude::*;
use diesel::r2d2;
use failure::Error;
use regex::Regex;
use textnonce::TextNonce;

use super::models;
use super::schema;

type Result<T> = ::std::result::Result<T, Error>;
pub type Pool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;
pub type Connection = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;

// Diesel does not have a `lower` function built in; create one ourselves.
// See: https://github.com/diesel-rs/diesel/issues/560#issuecomment-270199166
sql_function!(lower, lower_t, (a: diesel::types::VarChar) -> diesel::types::VarChar);

pub enum Queries {
    CreateDrink,
    GetDrinks,
    GetDrink,
    DeleteDrink,
    GetBreweryByName,
    GetBeerByName,
    CreateBrewery,
    CreateBeer,
    LookupIdentity,
    StartSession,
    GetSession,
    GetLoggedInPerson,
    SearchBeerByName,
    SearchBreweryByName,
}

