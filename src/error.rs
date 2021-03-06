/*
 * This file is part of Telegram Event Bot.
 *
 * Copyright © 2018 Riley Trautman
 *
 * Telegram Event Bot is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Telegram Event Bot is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Telegram Event Bot.  If not, see <http://www.gnu.org/licenses/>.
 */

//! This module describes the error types that are present in the event bot

use std::fmt;

use failure::{Backtrace, Context, Fail};

/// Wrap all errors that could happen in this application
#[derive(Debug)]
pub struct EventError {
    pub context: Context<EventErrorKind>,
}

impl Fail for EventError {
    fn cause(&self) -> Option<&Fail> {
        self.context.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.context.backtrace()
    }
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.context, f)
    }
}

impl From<EventErrorKind> for EventError {
    fn from(e: EventErrorKind) -> Self {
        EventError {
            context: Context::new(e),
        }
    }
}

impl From<Context<EventErrorKind>> for EventError {
    fn from(e: Context<EventErrorKind>) -> Self {
        EventError { context: e }
    }
}

/// Express the kinds of errors that happen in this application
#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum EventErrorKind {
    #[fail(display = "Failed to create a connection to the database")]
    CreateConnection,
    #[fail(display = "Failed to get environment variable")]
    MissingEnv,
    #[fail(display = "Failed to lookup data from db")]
    Lookup,
    #[fail(display = "Failed to prepare db query")]
    Prepare,
    #[fail(display = "Failed to insert item")]
    Insert,
    #[fail(display = "Failed to delete item")]
    Delete,
    #[fail(display = "Failed to update item")]
    Update,
    #[fail(display = "Failed to commit transaction")]
    Commit,
    #[fail(display = "Failed to create transaction")]
    Transaction,
    #[fail(display = "No hosts present")]
    Hosts,
    #[fail(display = "Failed passing message")]
    Canceled,
    #[fail(display = "Failed to send telegram message")]
    Telegram,
    #[fail(display = "Failed to lookup telegram item")]
    TelegramLookup,
    #[fail(display = "Error on frontend")]
    Frontend,
    #[fail(display = "User is not allowed to perform that action")]
    Permissions,
    #[fail(display = "Bad client secret")]
    Secret,
}

/// Provide an error type for missing keys when constructing the database URL
#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum DbConnError {
    #[fail(display = "Database username not supplied")]
    User,
    #[fail(display = "Database password not supplied")]
    Pass,
    #[fail(display = "Database host not supplied")]
    Host,
    #[fail(display = "Database port not supplied")]
    Port,
    #[fail(display = "Database name not supplied")]
    Name,
}
