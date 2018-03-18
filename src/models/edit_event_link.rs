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

//! This module defines the `EditEventLink` type, and associated types and functions

use futures::Future;
use futures_state_stream::StateStream;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use util::*;

/// `EditEventLink` defines generated links that are used to edit events. Users who host events
/// have permission to edit events, and these links ensure a one-time use edit is possible.
///
/// `user_id` is the database ID of the user who asked for this link
/// `system_id` is the database ID of the system the event is associated with
/// `event_id` is the database ID of the event this link is associated with
/// `secret` is a bcrypted secret used to verify that an edited event is valid
///
/// ### Relations:
/// - edit_event_links belongs_to users (foreign_key on edit_event_links)
/// - edit_event_links belongs_to chat_systems (foreign_key on edit_event_links)
/// - edit_event_links belongs_to events (foreign_key on edit_event_links)
///
/// ### Columns:
///  - id SERIAL
///  - user_id INTEGER REFERENCES users
///  - system_id INTEGER REFERENCES chat_systems
///  - event_id INTEGER REFERENCES events
///  - secret - TEXT
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditEventLink {
    id: i32,
    user_id: i32,
    system_id: i32,
    event_id: i32,
    secret: String,
}

impl EditEventLink {
    /// Get the ID of the `EditEventLink`
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the `User` database ID of the `EditEventLink`
    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    /// Get the `ChatSystem` database ID of the `EditEventLink`
    pub fn system_id(&self) -> i32 {
        self.system_id
    }

    /// Get the `Event` database ID of the `EditEventLink`
    pub fn event_id(&self) -> i32 {
        self.event_id
    }

    /// Get the secret from the `EditEventLink`
    ///
    /// TODO: Maybe don't do it like this, put verfication in `EditEventLink`?
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Insert an `EditEventLink` into the database given the associated IDs and the secret
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

    /// Lookup an `EditEventLink` by it's ID
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

    /// Mark an `EditEventLink` as used
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
