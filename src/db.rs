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
    type Result: Send;

    fn execute(&self, conn: Connection) -> Self::Result;
}

pub fn execute<T: Query + Send + 'static>(
    pool: &Pool,
    query: T,
) -> impl Future<Item = T::Result, Error = Error> {
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
    type Result = Result<models::Drink>;

    fn execute(&self, conn: Connection) -> Self::Result {
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
    type Result = Result<Vec<ExpandedDrink>>;

    fn execute(&self, conn: Connection) -> Self::Result {
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

/*************************************/
/** Get Drink message               **/
/*************************************/

pub struct GetDrink {
    pub drink_id: i32,
}

impl Query for GetDrink {
    type Result = Result<ExpandedDrink>;

    fn execute(&self, conn: Connection) -> Self::Result {
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
            .filter(drink::id.eq(&self.drink_id))
            .first::<ExpandedDrink>(&conn)?)
    }
}

/*************************************/
/** Delete Drink message            **/
/*************************************/

pub struct DeleteDrink {
    pub drink_id: i32,
    pub person_id: i32,
}

impl Query for DeleteDrink {
    type Result = Result<usize>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::drink::dsl::*;

        Ok(diesel::delete(
            drink.filter(id.eq(self.drink_id).and(person_id.eq(self.person_id))),
        )
        .execute(&conn)?)
    }
}

/*************************************/
/*************************************/

pub struct GetBreweryByName {
    pub name: String,
}

impl Query for GetBreweryByName {
    type Result = Result<Option<models::Brewery>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::brewery::dsl::*;

        Ok(brewery
            .filter(lower(name).eq(&self.name.to_lowercase()))
            .first::<models::Brewery>(&conn)
            .optional()?)
    }
}

/*************************************/
/*************************************/

pub struct GetBeerByName {
    pub name: String,
    pub brewery_id: i32,
}

impl Query for GetBeerByName {
    type Result = Result<Option<models::Beer>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::beer::dsl::*;

        Ok(beer
            .filter(
                lower(name)
                    .eq(&self.name.to_lowercase())
                    .and(brewery_id.eq(&self.brewery_id)),
            )
            .first::<models::Beer>(&conn)
            .optional()?)
    }
}

/*************************************/
/*************************************/

pub struct CreateBrewery {
    pub name: String,
}

impl Query for CreateBrewery {
    type Result = Result<models::Brewery>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::brewery::dsl::*;

        let new_brewery = models::NewBrewery {
            name: &self.name,
        };

        Ok(diesel::insert_into(brewery)
            .values(new_brewery)
            .get_result(&conn)?)
    }
}

/*************************************/
/*************************************/

pub struct CreateBeer {
    pub name: String,
    pub brewery_id: i32,
}

impl Query for CreateBeer {
    type Result = Result<models::Beer>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::beer::dsl::*;

        let new_beer = models::NewBeer {
            name: &self.name,
            brewery_id: self.brewery_id,
        };

        Ok(diesel::insert_into(beer)
            .values(new_beer)
            .get_result(&conn)?)
    }
}

/*************************************/
/* Login and Registration            */
/*************************************/

pub struct LookupIdentiy {
    pub identifier: String,
}

impl Query for LookupIdentiy {
    type Result = Result<models::Identity>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use self::schema::identity::dsl::*;
        use self::schema::person::dsl::*;

        // Query to see if a matching Identity exists
        let existing_identity = identity
            .filter(identifier.eq(&self.identifier))
            .first::<models::Identity>(&conn)
            .optional()?;

        // If an Identity was found, return it
        if let Some(existing_identity) = existing_identity {
            info!(
                "Found existing identity matching '{}', person {}.",
                existing_identity.identifier, existing_identity.person_id
            );
            return Ok(existing_identity);
        }

        // If here, then no identity was found
        // Create a new person to go with that identity
        let new_person = diesel::insert_into(person) /* lol */
            .default_values() // currently no other values need to be inserted at the moment
            .get_result::<models::Person>(&conn)?;

        // Insert the new identity
        let new_identity = diesel::insert_into(identity)
            .values(&models::NewIdentity {
                identifier: &self.identifier,
                person_id: new_person.id,
            })
            .get_result::<models::Identity>(&conn)?;

        info!(
            "Created new identity matching '{}' for person {}.",
            new_identity.identifier, new_identity.person_id
        );

        Ok(new_identity)
    }
}

/*************************************/

/*************************************/

pub struct StartSession {
    pub person_id: i32,
}

impl Query for StartSession {
    type Result = Result<models::Session>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use self::schema::login_session::dsl::*;

        // Create a unique identifier for this session
        let nonce = TextNonce::sized(64).unwrap();

        let new_session = models::NewSession {
            id: &nonce,
            person_id: self.person_id,
            expires_at: Utc::now() + Duration::weeks(2),
        };

        Ok(diesel::insert_into(login_session)
            .values(&new_session)
            .get_result::<models::Session>(&conn)?)
    }
}

/********************************/
/** Get Session                **/
/********************************/

pub struct GetSession {
    pub session_id: String,
}

impl Query for GetSession {
    type Result = Result<models::Session>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use self::schema::login_session::dsl::*;

        Ok(login_session
            .filter(id.eq(&self.session_id))
            .first::<models::Session>(&conn)?)
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
    type Result = Result<models::Person>;

    fn execute(&self, conn: Connection) -> Self::Result {
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

/*************************************/
/*************************************/

#[derive(Serialize, Queryable)]
#[serde(rename = "beers")]
pub struct BeerSearchResult {
    pub id: i32,
    pub name: String,
    pub brewery: String,
    pub rank: f32,
}

pub struct SearchBeerByName {
    pub query: String,
}

impl Query for SearchBeerByName {
    type Result = Result<Vec<BeerSearchResult>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::beer;
        use super::schema::beer::dsl::*;
        use super::schema::brewery;
        use diesel::dsl::sql;
        use diesel::sql_types::{Float, Text};

        let tsquery = tsquery_string(&self.query);

        let full_name_rank = sql::<Float>(&format!(
            r#"
            ts_rank(
                setweight(to_tsvector('english', beer.name), 'A') || 
                setweight(to_tsvector('english', brewery.name), 'B'),
                to_tsquery('english', '{}')
            ) as rank
        "#,
            &tsquery
        ));

        Ok(beer
            .inner_join(brewery::table)
            .group_by((beer::id, brewery::id))
            .select((beer::id, beer::name, brewery::name, full_name_rank))
            .order_by(sql::<Text>("rank").desc())
            .get_results(&conn)?)
    }
}

/*************************************/
/*************************************/

#[derive(Serialize, Queryable)]
#[serde(rename = "breweries")]
pub struct BrewerySearchResult {
    pub id: i32,
    pub name: String,
    pub rank: f32,
}

pub struct SearchBreweryByName {
    pub query: String,
}

impl Query for SearchBreweryByName {
    type Result = Result<Vec<BrewerySearchResult>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::brewery;
        use super::schema::brewery::dsl::*;
        use diesel::dsl::sql;
        use diesel::sql_types::{Float, Text};

        let tsquery = tsquery_string(&self.query);

        let full_name_rank = sql::<Float>(&format!(
            r#"
            ts_rank(
                setweight(to_tsvector('english', brewery.name), 'A'),
                to_tsquery('english', '{}')
            ) as rank
        "#,
            &tsquery
        ));

        Ok(brewery
            .select((brewery::id, brewery::name, full_name_rank))
            .order_by(sql::<Text>("rank").desc())
            .get_results(&conn)?)
    }
}

/// Remove all characters that are not alphanumeric or hyphens
/// then generate a string that may be used in `to_tsquery`.
///
/// Each word will be separated have have ":*" appended,
/// and then joined into a string with all words separated by " <-> ".
fn tsquery_string(text: &str) -> String {
    lazy_static! {
        static ref NON_ALPHANUMERIC: Regex = Regex::new(r"[^\w\s-]").unwrap();

        // This query attempts to permit hypens that actually separate word groups,
        // without permitting multiple hypens in a row that might result in SQLi.
        // It's basically selecting one or more alphanumeric groups, possibly followed by a hypen
        static ref TEXT_GROUPS: Regex = Regex::new(r"((?:[\w]+\-?)+\w{0,})").unwrap();
    }

    let cleaned = NON_ALPHANUMERIC.replace_all(text, "");
    TEXT_GROUPS
        .captures_iter(&cleaned)
        .map(|cap| format!("{}:*", &cap[1]))
        .collect::<Vec<String>>()
        .join(" <-> ")
}

#[cfg(test)]
mod tests {
    use super::tsquery_string;

    #[test]
    fn test_tsquery_string() {
        assert_eq!("test:* <-> beer:*", tsquery_string("test beer"));
        assert_eq!("test-beer:*", tsquery_string("test-beer"));
        assert_eq!("test:* <-> beer:*", tsquery_string(r#"test "'/#-- beer"#));
        assert_eq!(
            "another-test-beer:*",
            tsquery_string(r#"another-test-beer"#)
        );

        assert_eq!("test-:* <-> beer:*", tsquery_string("test--beer"));
        assert_eq!("test-:*", tsquery_string("test--"));
        assert_eq!("test-:*", tsquery_string("test-?-"));
    }
}
