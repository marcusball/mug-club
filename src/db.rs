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
