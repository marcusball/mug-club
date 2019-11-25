use actix_web::error::ResponseError;
use actix_web::Error as ActixError;
use authy::AuthyError;
use diesel::r2d2;
use diesel::result::Error as DieselError;
use futures::channel::oneshot::Canceled as FutureCanceled;
use std::convert::From;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Display)]
pub enum Error {
    ActixError,

    AuthyError(AuthyError),

    SessionNotFound,

    DieselError(DieselError),

    PoolError(r2d2::PoolError),

    FutureCanceled(FutureCanceled),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Self::ActixError => "Internal Actix Error",
            Self::AuthyError(e) => e.description(),
            Self::SessionNotFound => "Session not found!",
            Self::DieselError(e) => e.description(),
            Self::PoolError(e) => e.description(),
            Self::FutureCanceled(e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ActixError => None,
            Self::AuthyError(e) => Some(e),
            Self::DieselError(e) => Some(e),
            Self::PoolError(e) => Some(e),
            Self::FutureCanceled(e) => Some(e),
            Self::SessionNotFound => None,
        }
    }
}
impl ResponseError for Error {}

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

impl From<FutureCanceled> for Error {
    fn from(e: FutureCanceled) -> Error {
        Error::FutureCanceled(e)
    }
}

impl From<AuthyError> for Error {
    fn from(e: AuthyError) -> Error {
        Error::AuthyError(e)
    }
}

impl From<ActixError> for Error {
    fn from(e: ActixError) -> Error {
        Error::ActixError
    }
}
