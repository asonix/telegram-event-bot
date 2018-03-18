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

//! This module contains funtions for preparing for database interaction

use std::env;

use dotenv::dotenv;
use failure::{Context, Fail, ResultExt};
use futures::Future;
use tokio_core::reactor::Handle;
use tokio_postgres::{Connection, TlsMode};

use error::{DbConnError, EventError, EventErrorKind};

/// Wrap the var -> error -> context pipeline in a function
fn get_db_env(key: &str, err: DbConnError) -> Result<String, Context<EventErrorKind>> {
    env::var(key)
        .map_err(|_| err)
        .context(EventErrorKind::MissingEnv)
}

/// Build the database URL string from the provided environment variables
pub fn prepare_database_connection() -> Result<String, EventError> {
    dotenv().ok();

    let username = get_db_env("DB_USER", DbConnError::User)?;
    let password = get_db_env("DB_PASS", DbConnError::Pass)?;
    let host = get_db_env("DB_HOST", DbConnError::Host)?;
    let port = get_db_env("DB_PORT", DbConnError::Port)?;
    #[cfg(not(test))]
    let name = get_db_env("DB_NAME", DbConnError::Name)?;
    #[cfg(test)]
    let name = get_db_env("TEST_DB_NAME", DbConnError::Name)?;

    Ok(format!(
        "postgres://{}:{}@{}:{}/{}",
        username, password, host, port, name
    ))
}

/// Given a string, return a future representing the Database Connection
pub fn connect_to_database(
    db_url: String,
    handle: Handle,
) -> impl Future<Item = Connection, Error = EventError> {
    Connection::connect(db_url.as_ref(), TlsMode::None, &handle)
        .map_err(|e| e.context(EventErrorKind::CreateConnection).into())
}
