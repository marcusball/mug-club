extern crate actix;

use actix::prelude::*;
use chrono::naive::NaiveDate;
use chrono::{Duration, Utc};
use diesel;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::Error;
use regex::Regex;
use textnonce::TextNonce;

use super::models;
use super::schema;

type Result<T> = ::std::result::Result<T, Error>;

// Diesel does not have a `lower` function built in; create one ourselves.
// See: https://github.com/diesel-rs/diesel/issues/560#issuecomment-270199166
sql_function!(lower, lower_t, (a: diesel::types::VarChar) -> diesel::types::VarChar);

pub struct DatabaseExecutor(pub Pool<ConnectionManager<PgConnection>>);

impl DatabaseExecutor {
    fn get_conn<'a>(
        &mut self,
    ) -> Result<diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>> {
        self.0.get().map_err(|e| Error::from(e))
    }
}

impl Actor for DatabaseExecutor {
    type Context = SyncContext<Self>;
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

impl Message for CreateDrink {
    type Result = Result<models::Drink>;
}

impl Handler<CreateDrink> for DatabaseExecutor {
    type Result = Result<models::Drink>;

    fn handle(&mut self, message: CreateDrink, _: &mut Self::Context) -> Self::Result {
        use self::schema::drink::dsl::*;

        let conn = self.get_conn()?;

        let new_drink = models::NewDrink {
            person_id: &message.person_id,
            drank_on: &message.drank_on,
            beer_id: &message.beer_id,
            rating: &message.rating,
            comment: message.comment.as_ref(),
        };

        Ok(diesel::insert_into(drink)
            .values(&new_drink)
            .get_result(&conn)?)
    }
}

/*************************************/
/** Get Drinks message              **/
/*************************************/

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

pub struct GetDrinks {
    pub person_id: i32,
}

impl Message for GetDrinks {
    type Result = Result<Vec<ExpandedDrink>>;
}

impl Handler<GetDrinks> for DatabaseExecutor {
    type Result = Result<Vec<ExpandedDrink>>;

    fn handle(&mut self, message: GetDrinks, _: &mut Self::Context) -> Self::Result {
        use super::schema::beer;
        use super::schema::beer::dsl::*;
        use super::schema::brewery;
        use super::schema::drink;
        use super::schema::drink::dsl::*;

        let conn = self.get_conn()?;

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
            .filter(drink::person_id.eq(&message.person_id))
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

impl Message for GetDrink {
    type Result = Result<ExpandedDrink>;
}

impl Handler<GetDrink> for DatabaseExecutor {
    type Result = Result<ExpandedDrink>;

    fn handle(&mut self, message: GetDrink, _: &mut Self::Context) -> Self::Result {
        use super::schema::beer;
        use super::schema::beer::dsl::*;
        use super::schema::brewery;
        use super::schema::drink;
        use super::schema::drink::dsl::*;

        let conn = self.get_conn()?;

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
            .filter(drink::id.eq(&message.drink_id))
            .first::<ExpandedDrink>(&conn)?)
    }
}

/*************************************/
/*************************************/

pub struct GetBreweryByName {
    pub name: String,
}

impl Message for GetBreweryByName {
    type Result = Result<Option<models::Brewery>>;
}

impl Handler<GetBreweryByName> for DatabaseExecutor {
    type Result = Result<Option<models::Brewery>>;

    fn handle(&mut self, message: GetBreweryByName, _: &mut Self::Context) -> Self::Result {
        use super::schema::brewery::dsl::*;

        let conn = self.get_conn()?;

        Ok(brewery
            .filter(lower(name).eq(&message.name.to_lowercase()))
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

impl Message for GetBeerByName {
    type Result = Result<Option<models::Beer>>;
}

impl Handler<GetBeerByName> for DatabaseExecutor {
    type Result = Result<Option<models::Beer>>;

    fn handle(&mut self, message: GetBeerByName, _: &mut Self::Context) -> Self::Result {
        use super::schema::beer::dsl::*;

        let conn = self.get_conn()?;

        Ok(beer
            .filter(
                lower(name)
                    .eq(&message.name.to_lowercase())
                    .and(brewery_id.eq(&message.brewery_id)),
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

impl Message for CreateBrewery {
    type Result = Result<models::Brewery>;
}

impl Handler<CreateBrewery> for DatabaseExecutor {
    type Result = Result<models::Brewery>;

    fn handle(&mut self, message: CreateBrewery, _: &mut Self::Context) -> Self::Result {
        use super::schema::brewery::dsl::*;

        let conn = self.get_conn()?;

        let new_brewery = models::NewBrewery {
            name: &message.name,
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

impl Message for CreateBeer {
    type Result = Result<models::Beer>;
}

impl Handler<CreateBeer> for DatabaseExecutor {
    type Result = Result<models::Beer>;

    fn handle(&mut self, message: CreateBeer, _: &mut Self::Context) -> Self::Result {
        use super::schema::beer::dsl::*;

        let conn = self.get_conn()?;

        let new_beer = models::NewBeer {
            name: &message.name,
            brewery_id: message.brewery_id,
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

impl Message for LookupIdentiy {
    type Result = Result<models::Identity>;
}

impl Handler<LookupIdentiy> for DatabaseExecutor {
    type Result = Result<models::Identity>;

    fn handle(&mut self, message: LookupIdentiy, _: &mut Self::Context) -> Self::Result {
        use self::schema::identity::dsl::*;
        use self::schema::person::dsl::*;

        let conn = self.get_conn()?;

        // Query to see if a matching Identity exists
        let existing_identity = identity
            .filter(identifier.eq(&message.identifier))
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
                identifier: &message.identifier,
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

impl Message for StartSession {
    type Result = Result<models::Session>;
}

impl Handler<StartSession> for DatabaseExecutor {
    type Result = Result<models::Session>;

    fn handle(&mut self, message: StartSession, _: &mut Self::Context) -> Self::Result {
        use self::schema::login_session::dsl::*;

        let conn = self.get_conn()?;

        // Create a unique identifier for this session
        let nonce = TextNonce::sized(64).unwrap();

        let new_session = models::NewSession {
            id: &nonce,
            person_id: message.person_id,
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

impl Message for GetSession {
    type Result = Result<models::Session>;
}

impl Handler<GetSession> for DatabaseExecutor {
    type Result = Result<models::Session>;

    fn handle(&mut self, message: GetSession, _: &mut Self::Context) -> Self::Result {
        use self::schema::login_session::dsl::*;

        let conn = self.get_conn()?;

        Ok(login_session
            .filter(id.eq(&message.session_id))
            .first::<models::Session>(&conn)?)
    }
}

/********************************/
/** Get Logged-in Person       **/
/********************************/

/// This is a `Message` for getting the current active user
/// given the peron's `session_id`.
pub struct GetLoggedInPerson {
    pub session_id: String,
}

impl Message for GetLoggedInPerson {
    type Result = Result<models::Person>;
}

impl Handler<GetLoggedInPerson> for DatabaseExecutor {
    type Result = Result<models::Person>;

    fn handle(&mut self, message: GetLoggedInPerson, _: &mut Self::Context) -> Self::Result {
        use self::schema::login_session::dsl::id as sid;
        use self::schema::login_session::dsl::login_session;
        use self::schema::person::dsl::*;

        let conn = self.get_conn()?;

        Ok(person
            .inner_join(login_session)
            .filter(sid.eq(&message.session_id))
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

impl Message for SearchBeerByName {
    type Result = Result<Vec<BeerSearchResult>>;
}

impl Handler<SearchBeerByName> for DatabaseExecutor {
    type Result = Result<Vec<BeerSearchResult>>;

    fn handle(&mut self, message: SearchBeerByName, _: &mut Self::Context) -> Self::Result {
        use super::schema::beer;
        use super::schema::beer::dsl::*;
        use super::schema::brewery;
        use diesel::dsl::sql;
        use diesel::sql_types::{Float, Text};

        let conn = self.get_conn()?;

        let tsquery = tsquery_string(&message.query);

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

impl Message for SearchBreweryByName {
    type Result = Result<Vec<BrewerySearchResult>>;
}

impl Handler<SearchBreweryByName> for DatabaseExecutor {
    type Result = Result<Vec<BrewerySearchResult>>;

    fn handle(&mut self, message: SearchBreweryByName, _: &mut Self::Context) -> Self::Result {
        use super::schema::brewery;
        use super::schema::brewery::dsl::*;
        use diesel::dsl::sql;
        use diesel::sql_types::{Float, Text};

        let conn = self.get_conn()?;

        let tsquery = tsquery_string(&message.query);

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
