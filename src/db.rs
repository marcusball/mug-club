use actix_web::web;
use actix_web::Error as AWError;
use chrono::naive::NaiveDate;
use chrono::{Duration, Utc};
use diesel;
use diesel::prelude::*;
use diesel::r2d2;
use futures::future::Future;
use regex::Regex;
use textnonce::TextNonce;

use std::marker::Send;

use super::error::{Error, Result};
use super::models;
use super::schema;

pub type Pool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;
pub type Connection = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;

// Diesel does not have a `lower` function built in; create one ourselves.
// See: https://github.com/diesel-rs/diesel/issues/560#issuecomment-270199166
sql_function!(lower, lower_t, (a: diesel::types::VarChar) -> diesel::types::VarChar);

pub trait Query {
    type Item: Send;

    fn execute(&self, conn: Connection) -> Self::Item;
}

pub fn execute<T: Query + Send + Clone + 'static>(
    pool: &Pool,
    query: T,
) -> impl Future<Item = T::Item, Error = Error> {
    let pool = pool.clone();

    web::block::<_, _, Error>(move || Ok(query.execute(pool.get()?))).from_err()
}

#[derive(Serialize, Queryable)]
#[serde(rename = "drink")]
pub struct ExpandedDrink {
    pub id: i32,
    pub drank_on: NaiveDate,
    pub name: String,
    pub brewery: String,
    pub rating: i16,
    pub comment: Option<String>,
}

/*************************************/
/** Create Drink message            **/
/*************************************/

pub struct CreateDrink {
    pub person_id: i32,
    pub drank_on: NaiveDate,
    pub beer_id: i32,
    pub rating: i16,
    pub comment: Option<String>,
}

impl Query for CreateDrink {
    type Item = Result<models::Drink>;

    fn execute(&self, conn: Connection) -> Self::Item {
        use self::schema::drink::dsl::*;

        let new_drink = models::NewDrink {
            person_id: &self.person_id,
            drank_on: &self.drank_on,
            beer_id: &self.beer_id,
            rating: &self.rating,
            comment: self.comment.as_ref(),
        };

        Ok(diesel::insert_into(drink)
            .values(&new_drink)
            .get_result(&conn)?)
    }
}

/*************************************/
/** Get Drinks query                **/
/*************************************/

#[derive(Clone)]
pub struct GetDrinks {
    pub person_id: i32,
}

impl Query for GetDrinks {
    type Item = Result<Vec<ExpandedDrink>>;

    fn execute(&self, conn: Connection) -> Self::Item {
        use super::schema::beer;
        use super::schema::beer::dsl::*;
        use super::schema::brewery;
        use super::schema::drink;
        use super::schema::drink::dsl::*;

        Ok(drink
            .inner_join(beer)
            .inner_join(brewery::table.on(beer::brewery_id.eq(brewery::id)))
            .select((
                drink::id,
                drink::drank_on,
                beer::name,
                brewery::name,
                drink::rating,
                drink::comment,
            ))
            .filter(drink::person_id.eq(&self.person_id))
            .order(drink::drank_on.asc())
            .load::<ExpandedDrink>(&conn)?)
    }
}

/********************************/
/** Get Logged-in Person       **/
/********************************/

/// This is a `Message` for getting the current active user
/// given the peron's `session_id`.
#[derive(Clone)]
pub struct GetLoggedInPerson {
    pub session_id: String,
}

impl GetLoggedInPerson {
    pub fn from_session(session_id: String) -> GetLoggedInPerson {
        GetLoggedInPerson { session_id }
    }
}

impl Query for GetLoggedInPerson {
    type Item = Result<models::Person>;

    fn execute(&self, conn: Connection) -> Self::Item {
        use self::schema::login_session::dsl::id as sid;
        use self::schema::login_session::dsl::login_session;
        use self::schema::person::dsl::*;

        Ok(person
            .inner_join(login_session)
            .filter(sid.eq(&self.session_id))
            .select((id, created_at, updated_at))
            .first::<models::Person>(&conn)?)
    }
}
