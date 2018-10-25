extern crate actix;

use actix::prelude::*;
use chrono::naive::NaiveDate;
use diesel;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::Error;

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
pub struct ExpandedDrink {
    pub id: i32,
    pub drank_on: NaiveDate,
    pub beer: String,
    pub brewery: String,
    pub rating: i16,
    pub comment: Option<String>,
}

pub struct GetDrinks;

impl Message for GetDrinks {
    type Result = Result<Vec<ExpandedDrink>>;
}

impl Handler<GetDrinks> for DatabaseExecutor {
    type Result = Result<Vec<ExpandedDrink>>;

    fn handle(&mut self, _: GetDrinks, _: &mut Self::Context) -> Self::Result {
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
            .load::<ExpandedDrink>(&conn)?)
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

/*
/// Returns the person record, if one already exists,
/// otherwise creates a new person record.
fn get_or_register_person(message: &CreatePerson, conn: &PgConnection) -> Result<models::Person> {
    use self::schema::person::dsl::*;

    let uuid = Uuid::new_v4();
    let new_person = models::NewPerson {
        id: &uuid,
        name: &message.name,
    };

    // Query to see if a person with this identity already exists
    let existing_person = person
        .filter(name.eq(&message.name))
        .first::<models::Person>(conn)
        .optional();

    // If this is not a new user, return the existing identity
    if let Ok(Some(login)) = existing_person {
        return Ok(login);
    }

    // Otherwise, register the new person
    Ok(diesel::insert_into(person)
        .values(&new_person)
        .get_result(conn)
        .unwrap())
}

fn create_login_session(person: &models::Person, conn: &PgConnection) -> Result<models::Session> {
    use self::schema::login_session::dsl::*;

    // Create a unique identifier for this session
    let nonce = TextNonce::sized(64).unwrap();

    let new_session = models::NewSession {
        id: &nonce,
        person_id: &person.id,
        expires_at: Utc::now() + Duration::weeks(2),
    };

    Ok(diesel::insert_into(login_session)
        .values(&new_session)
        .get_result(conn)
        .unwrap())
}
*/
