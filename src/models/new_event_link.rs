use futures::Future;
use futures_state_stream::StateStream;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use super::util::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewEventLink {
    id: i32,
    secret: String,
}

impl NewEventLink {
    pub fn create(
        user_id: i32,
        secret: String,
        connection: Connection,
    ) -> impl Future<Item = (Self, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO new_event_links (users_id, secret) VALUES ($1, $2) RETURNING id";

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&user_id, &secret])
                    .map(move |row| NewEventLink {
                        id: row.get(0),
                        secret: secret.clone(),
                    })
                    .collect()
                    .map_err(insert_error)
                    .and_then(|(mut nels, connection)| {
                        if nels.len() > 0 {
                            Ok((nels.remove(0), connection))
                        } else {
                            Err((EventErrorKind::Insert.into(), connection))
                        }
                    })
            })
    }

    pub fn by_user_id(
        user_id: i32,
        connection: Connection,
    ) -> impl Future<Item = (Self, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT nel.id, nel.secret
                    FROM new_event_links AS nel
                    WHERE nel.users_id = $1";

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&user_id])
                    .map(|row| NewEventLink {
                        id: row.get(0),
                        secret: row.get(1),
                    })
                    .collect()
                    .map_err(lookup_error)
                    .and_then(|(mut nels, connection)| {
                        if nels.len() > 0 {
                            Ok((nels.remove(0), connection))
                        } else {
                            Err((EventErrorKind::Lookup.into(), connection))
                        }
                    })
            })
    }

    pub fn delete(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = Connection, Error = (EventError, Connection)> {
        let sql = "DELETE FROM new_event_links WHERE id = $1";

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .execute(&s, &[&id])
                    .map_err(delete_error)
                    .and_then(|(count, connection)| {
                        if count > 0 {
                            Ok(connection)
                        } else {
                            Err((EventErrorKind::Delete.into(), connection))
                        }
                    })
            })
    }
}
