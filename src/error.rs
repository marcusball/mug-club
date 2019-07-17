use actix_web::Error as ActixError;
use actix_web::error::{ResponseError, BlockingError};
use diesel::result::Error as DieselError;
use diesel::r2d2;
use std::convert::From;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Display)]
pub enum Error {
    SessionNotFound,

    BlockingError,

    DieselError(DieselError),

    PoolError(r2d2::PoolError)
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Self::SessionNotFound => "Session not found!",
            Self::BlockingError => "Blocking Error!",
            Self::DieselError(e) => e.description(),
            Self::PoolError(e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DieselError(e) => Some(e),
            Self::PoolError(e) => Some(e),
            Self::SessionNotFound => None,
            Self::BlockingError => None
        }
    }
}


impl ResponseError for Error {}

impl<E> From<BlockingError<E>> for Error where E: std::fmt::Debug {
    fn from(e: BlockingError<E>) -> Error {
        Error::BlockingError
    }
}

impl From<DieselError> for Error {
    fn from(e: DieselError) -> Error {
        Error::DieselError(e)
    }
}

impl From<r2d2::PoolError> for Error {
    fn from(e: r2d2::PoolError) -> Error {
        Error::PoolError(e)
    }
}