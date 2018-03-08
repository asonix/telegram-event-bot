use futures::Future;
use futures_state_stream::StateStream;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use util::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewEventLink {
    id: i32,
    user_id: i32,
    system_id: i32,
    secret: String,
}

impl NewEventLink {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    pub fn system_id(&self) -> i32 {
        self.system_id
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn create(
        user_id: i32,
        system_id: i32,
        secret: String,
        connection: Connection,
    ) -> impl Future<Item = (Self, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO new_event_links (users_id, system_id, secret) VALUES ($1, $2, $3) RETURNING id";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&user_id, &system_id, &secret])
                    .map(move |row| NewEventLink {
                        id: row.get(0),
                        user_id: user_id,
                        system_id: system_id,
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

    pub fn by_id(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (Self, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT nel.id, nel.users_id, nel.system_id, nel.secret
                    FROM new_event_links AS nel
                    WHERE nel.id = $1 AND nel.used = FALSE";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&id])
                    .map(|row| NewEventLink {
                        id: row.get(0),
                        user_id: row.get(1),
                        system_id: row.get(2),
                        secret: row.get(3),
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
        let sql = "UPDATE new_event_links SET used = TRUE WHERE id = $1";
        debug!("{}", sql);

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
