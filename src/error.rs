use actix_web::error::ResponseError;
use actix_web::Error as ActixError;
use authy::AuthyError;
use diesel::r2d2;
use diesel::result::Error as DieselError;
use futures::channel::oneshot::Canceled as FutureCanceled;
use std::convert::From;
use std::error::Error as StdError;

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
impl ResponseError for Error {
    fn error_response(&self) -> actix_web::web::HttpResponse {
        actix_web::dev::HttpResponseBuilder::new(self.status_code())
            .set_header(
                actix_web::http::header::CONTENT_TYPE,
                "text/html; charset=utf-8",
            )
            .body(format!(
                "Error: {}\n\n{:?}",
                self.to_string(),
                self.source()
            ))
    }
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
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
