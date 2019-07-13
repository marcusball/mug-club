#![allow(proc_macro_derive_resolution_fallback)] // See: https://github.com/diesel-rs/diesel/issues/1785
extern crate chrono;

use actix_web::error as ActixError;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use chrono::naive::NaiveDate;
use chrono::{DateTime, Utc};
use crate::schema::*;
use futures::Future;

#[derive(Serialize, Queryable)]

/*************************************/
/* Brewery Models                    */
/*************************************/

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

/*************************************/
/* Beer Models                       */
/*************************************/

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

/*************************************/
/* Drink Models                      */
/*************************************/

#[derive(Serialize, Queryable)]
pub struct Drink {
    pub id: i32,
    pub person_id: i32,
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
    pub person_id: &'a i32,
    pub drank_on: &'a NaiveDate,
    pub beer_id: &'a i32,
    pub rating: &'a i16,
    pub comment: Option<&'a String>,
}

/*************************************/
/* Person Models                     */
/*************************************/

#[derive(Serialize, Queryable)]
pub struct Person {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Queryable)]
pub struct Identity {
    pub identifier: String,
    pub person_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "identity"]
pub struct NewIdentity<'a> {
    pub identifier: &'a str,
    pub person_id: i32,
}

#[derive(Serialize, Queryable)]
#[serde(rename = "session")]
pub struct Session {
    pub id: String,
    pub person_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "login_session"]
pub struct NewSession<'a> {
    pub id: &'a str,
    pub person_id: i32,
    pub expires_at: DateTime<Utc>,
}

/*********************/
/* Login Sessions    */
/*********************/