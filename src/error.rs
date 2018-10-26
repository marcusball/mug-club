use actix_web::error::ResponseError;
use failure::Fail;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Session not found")]
    SessionNotFound,
}

impl ResponseError for Error {}
