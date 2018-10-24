#![allow(proc_macro_derive_resolution_fallback)] // See: https://github.com/diesel-rs/diesel/issues/1785
extern crate chrono;

use super::schema::*;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Utc};

#[derive(Serialize, Queryable)]
pub struct Brewery {
    pub id: i32,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "brewery"]
pub struct NewBrewery<'a> {
    pub name: &'a str,
}

#[derive(Serialize, Queryable)]
pub struct Beer {
    pub id: i32,
    pub name: String,
    pub brewery_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "beer"]
pub struct NewBeer<'a> {
    pub name: &'a str,
    pub brewery_id: i32,
}

#[derive(Serialize, Queryable)]
pub struct Drink {
    pub id: i32,
    pub drank_on: NaiveDate,
    pub beer_id: i32,
    pub rating: i16,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "drink"]
pub struct NewDrink<'a> {
    pub drank_on: &'a NaiveDate,
    pub beer_id: &'a i32,
    pub rating: &'a i16,
    pub comment: Option<&'a String>,
}
