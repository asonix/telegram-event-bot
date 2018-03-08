use futures::Future;
use futures_state_stream::StateStream;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use util::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditEventLink {
    id: i32,
    user_id: i32,
    system_id: i32,
    event_id: i32,
    secret: String,
}

impl EditEventLink {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    pub fn system_id(&self) -> i32 {
        self.system_id
    }

    pub fn event_id(&self) -> i32 {
        self.event_id
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn create(
        user_id: i32,
        system_id: i32,
        event_id: i32,
        secret: String,
        connection: Connection,
    ) -> impl Future<Item = (Self, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO edit_event_links (users_id, system_id, events_id, secret) VALUES ($1, $2, $3, $4) RETURNING id";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&user_id, &system_id, &event_id, &secret])
                    .map(move |row| EditEventLink {
                        id: row.get(0),
                        user_id,
                        system_id,
                        event_id,
                        secret: secret.clone(),
                    })
                    .collect()
                    .map_err(insert_error)
                    .and_then(|(mut eels, connection)| {
                        if eels.len() > 0 {
                            Ok((eels.remove(0), connection))
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
        let sql = "SELECT eel.id, eel.users_id, eel.system_id, eel.events_id, eel.secret
                    FROM edit_event_links AS eel
                    WHERE eel.id = $1 AND eel.used = FALSE";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&id])
                    .map(|row| EditEventLink {
                        id: row.get(0),
                        user_id: row.get(1),
                        system_id: row.get(2),
                        event_id: row.get(3),
                        secret: row.get(4),
                    })
                    .collect()
                    .map_err(lookup_error)
                    .and_then(|(mut eels, connection)| {
                        if eels.len() > 0 {
                            Ok((eels.remove(0), connection))
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
        let sql = "UPDATE edit_event_links SET used = TRUE WHERE id = $1";
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
