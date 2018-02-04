use std::env;

use dotenv::dotenv;
use failure::{Context, Fail, ResultExt};
use futures::{Future, IntoFuture};
use tokio_core::reactor::Handle;
use tokio_postgres::{Connection, TlsMode};
use error::{DbConnError, EventError, EventErrorKind};

// Wrap the var -> error -> context pipeline in a function
fn get_db_env(key: &str, err: DbConnError) -> Result<String, Context<EventErrorKind>> {
    env::var(key)
        .map_err(|_| err)
        .context(EventErrorKind::MissingEnv)
}

// Build the database URL string from the provided environment variables
fn prepare_database_connection() -> Result<String, EventError> {
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

/// Given a handle to the event loop, create a connection to the database
pub fn database_connection(handle: Handle) -> Box<Future<Item = Connection, Error = EventError>> {
    Box::new(
        prepare_database_connection()
            .into_future()
            .and_then(move |db_url| {
                Connection::connect(db_url.as_ref(), TlsMode::None, &handle)
                    .map_err(|e| e.context(EventErrorKind::CreateConnection).into())
            }),
    )
}
