use std::fmt;

use actix_web::*;
use actix_web::error::ResponseError;
use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct FrontendError {
    context: Context<FrontendErrorKind>,
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.context.fmt(f)
    }
}

impl Fail for FrontendError {
    fn cause(&self) -> Option<&Fail> {
        self.context.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.context.backtrace()
    }
}

impl ResponseError for FrontendError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(
            StatusCode::BAD_REQUEST,
            format!("{}, {:?}", self, self).into(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum FrontendErrorKind {
    #[fail(display = "Error generating client secret")] Generation,
    #[fail(display = "Error verifying client secret")] Verification,
    #[fail(display = "Error generating response body")] Body,
    #[fail(display = "Error generating response")] Response,
    #[fail(display = "Missing a required field")] MissingField,
    #[fail(display = "Invalid timezone")] BadTimeZone,
    #[fail(display = "Invalid year")] BadYear,
    #[fail(display = "Invalid month")] BadMonth,
    #[fail(display = "Invalid day")] BadDay,
    #[fail(display = "Invalid hour")] BadHour,
    #[fail(display = "Invalid minute")] BadMinute,
    #[fail(display = "Invalid second")] BadSecond,
    #[fail(display = "Could not find requested route")] NoRoute,
    #[fail(display = "Could not interact with session")] Session,
    #[fail(display = "Message from backend canceled")] Canceled,
}

impl From<FrontendErrorKind> for FrontendError {
    fn from(e: FrontendErrorKind) -> Self {
        FrontendError {
            context: Context::new(e),
        }
    }
}

impl From<Context<FrontendErrorKind>> for FrontendError {
    fn from(e: Context<FrontendErrorKind>) -> Self {
        FrontendError { context: e }
    }
}

#[derive(Clone, Debug, Eq, Fail, PartialEq)]
#[fail(display = "Missing field {}", field)]
pub struct MissingField {
    pub field: &'static str,
}
