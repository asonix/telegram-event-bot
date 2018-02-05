use failure::Fail;
use tokio_postgres::{Connection, Error as TpError};
use tokio_postgres::transaction::Transaction;

use error::{EventError, EventErrorKind};

pub fn prepare_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Prepare).into(), connection)
}

pub fn insert_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Insert).into(), connection)
}

pub fn lookup_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Lookup).into(), connection)
}

pub fn delete_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Delete).into(), connection)
}

pub fn transaction_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (
        error.context(EventErrorKind::Transaction).into(),
        connection,
    )
}

pub fn transaction_prepare_error(
    (error, transaction): (TpError, Transaction),
) -> (EventError, Transaction) {
    (error.context(EventErrorKind::Prepare).into(), transaction)
}

pub fn transaction_insert_error(
    (error, transaction): (TpError, Transaction),
) -> (EventError, Transaction) {
    (error.context(EventErrorKind::Insert).into(), transaction)
}

pub fn commit_error((error, connection): (TpError, Connection)) -> (EventError, Connection) {
    (error.context(EventErrorKind::Commit).into(), connection)
}
