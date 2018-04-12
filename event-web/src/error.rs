/*
 * This file is part of Event Web
 *
 * Event Web is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Event Web is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with Event Web.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt;

use actix_web::*;
use actix_web::error::ResponseError;
use views::error;
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
        HttpResponse::BadRequest().body(error(self).into_string())
    }
}

#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum FrontendErrorKind {
    #[fail(display = "Error generating client secret")]
    Generation,
    #[fail(display = "Error verifying client secret")]
    Verification,
    #[fail(display = "Error generating response body")]
    Body,
    #[fail(display = "Error generating response")]
    Response,
    #[fail(display = "Missing a required field")]
    MissingField,
    #[fail(display = "Invalid timezone")]
    BadTimeZone,
    #[fail(display = "Invalid year")]
    BadYear,
    #[fail(display = "Invalid month")]
    BadMonth,
    #[fail(display = "Invalid day")]
    BadDay,
    #[fail(display = "Invalid hour")]
    BadHour,
    #[fail(display = "Invalid minute")]
    BadMinute,
    #[fail(display = "Invalid second")]
    BadSecond,
    #[fail(display = "Could not find requested route")]
    NoRoute,
    #[fail(display = "Could not interact with session")]
    Session,
    #[fail(display = "Message from backend canceled")]
    Canceled,
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
