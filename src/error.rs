use actix_web::Error as ActixError;
use actix_web::error::{ResponseError, BlockingError};
use failure::Fail;
use std::convert::From;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Session not found")]
    SessionNotFound,

    #[fail(display = "Blocking error")]
    BlockingError,

    #[fail(display = "Server error")]
    ActixError
}

impl ResponseError for Error {}

impl<E> From<BlockingError<E>> for Error where E: std::fmt::Debug {
    fn from(e: BlockingError<E>) -> Error {
        Error::BlockingError
    }
}
impl From<ActixError> for Error {
    fn from(e: ActixError) -> Error {
        Error::ActixError
    }
}