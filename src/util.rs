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

//! This module contains useful functions for handling errors from various sources
//!
//! Many of these functions are used in future with tokio_postgres. For example, the
//! `prepare_error` function can be used after calling `Connection::prepare(connection, sql)` to
//! translate the resulting tokio_postgres::Error into an error::Error

use actix::ResponseType;
use failure::Fail;
use futures::sync::oneshot::Canceled;
use tokio_postgres::{Connection, Error as TpError};
use tokio_postgres::transaction::Transaction;

use error::{EventError, EventErrorKind};

/// Convert a prepare error into an `EventError`
pub(crate) fn prepare_error(
    (error, connection): (TpError, Connection),
) -> (EventError, Connection) {
    (error.context(EventErrorKind::Prepare).into(), connection)
}

/// Convert an insert error into an `EventError`
pub(crate) fn insert_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Insert).into(), connection)
}

/// Convert a lookup error into an `EventError`
pub(crate) fn lookup_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Lookup).into(), connection)
}

/// Convert a delete error into an `EventError`
pub(crate) fn delete_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Delete).into(), connection)
}

/// Convert an update error into an `EventError`
pub(crate) fn update_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Update).into(), connection)
}

/// Convert a transaction error into an `EventError`
pub(crate) fn transaction_error(
    (error, connection): (TpError, Connection),
) -> (EventError, Connection) {
    (
        error.context(EventErrorKind::Transaction).into(),
        connection,
    )
}

/// Convert a transaction prepare error into an `EventError`
pub(crate) fn transaction_prepare_error(
    (error, transaction): (TpError, Transaction),
) -> (EventError, Transaction) {
    (error.context(EventErrorKind::Prepare).into(), transaction)
}

/// Convert a transaction insert error into an `EventError`
pub(crate) fn transaction_insert_error(
    (error, transaction): (TpError, Transaction),
) -> (EventError, Transaction) {
    (error.context(EventErrorKind::Insert).into(), transaction)
}

/// Convert a transaction lookup error into an `EventError`
pub(crate) fn transaction_lookup_error(
    (error, transaction): (TpError, Transaction),
) -> (EventError, Transaction) {
    (error.context(EventErrorKind::Lookup).into(), transaction)
}

/// Convert a transaction commit error into an `EventError`
pub(crate) fn commit_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Commit).into(), connection)
}

/// Flatten the result of a call to `addr.call_fut()` from a `Result<Result<_, _>, _>` into a
/// `Result<_, _>` by combining the error types
///
/// This can be used when chaining futures like `Address::call_fut(addr,
/// msg).then(flatten::<Msg>())`
pub(crate) fn flatten<T>(
    msg_res: Result<Result<T::Item, T::Error>, Canceled>,
) -> Result<T::Item, T::Error>
where
    T: ResponseType,
    T::Error: From<EventError>,
{
    match msg_res {
        Ok(res) => res,
        Err(e) => Err(EventError::from(e.context(EventErrorKind::Canceled)).into()),
    }
}
