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

//! This module defines a `ChatSystem` and related functions and types.
//!
//! Chat Systems are used to group a series of chats together with an events channel, to allow
//! members of those chats to create events for the channel

use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use super::chat::Chat;
use error::{EventError, EventErrorKind};
use util::*;

/// ChatSystem represents a series of linked chats
///
/// `events_channel` is the ID of the channel where full announcements are made
/// `announce_chats` is as set of IDs where the bot should notify of announcements.
///
/// This is represented in the database as
///
/// ### Relations:
/// - chat_systems has_many chats (foreign_key on chats)
///
/// ### Columns:
/// - id SERIAL
/// - events_channel BIGINT
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatSystem {
    id: i32,
    events_channel: Integer,
}

impl ChatSystem {
    /// Get the ID of the Chat System
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the Telegram Channel ID for the Chat System's channel
    pub fn events_channel(&self) -> Integer {
        self.events_channel
    }

    /// Create a `ChatSystem` given a Telegram Chat ID
    pub fn create(
        events_channel: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Self, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO chat_systems (events_channel) VALUES ($1) RETURNING id";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&events_channel])
                    .map(move |row| ChatSystem {
                        id: row.get(0),
                        events_channel: events_channel,
                    })
                    .collect()
                    .map_err(insert_error)
                    .and_then(|(mut chat_systems, connection)| {
                        if chat_systems.len() > 0 {
                            Ok((chat_systems.remove(0), connection))
                        } else {
                            Err((EventErrorKind::Insert.into(), connection))
                        }
                    })
            })
    }

    /// Fetch a chat system given it's ID
    pub fn by_id(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT sys.id, sys.events_channel
                    FROM chat_systems AS sys
                    WHERE sys.id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&id])
                    .map(|row| ChatSystem {
                        id: row.get(0),
                        events_channel: row.get(1),
                    })
                    .collect()
                    .map_err(lookup_error)
                    .and_then(|(mut chat_systems, connection)| {
                        if chat_systems.len() == 1 {
                            Ok((chat_systems.remove(0), connection))
                        } else {
                            Err((EventErrorKind::Lookup.into(), connection))
                        }
                    })
            })
    }

    /// Delete a `ChatSystem` and all associated `Chats`, `Events`, and `Users` given an id
    pub fn delete_by_id(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (u64, Connection), Error = (EventError, Connection)> {
        let sql = "DELETE FROM chat_systems AS sys WHERE sys.id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| connection.execute(&s, &[&id]).map_err(delete_error))
    }

    /// Delete a `ChatSystem` and all associated `Chats`, `Events`, and `Users`
    pub fn delete(
        self,
        connection: Connection,
    ) -> impl Future<Item = (u64, Connection), Error = (EventError, Connection)> {
        ChatSystem::delete_by_id(self.id, connection)
    }

    /// Select the chat system by channel id
    pub fn by_channel_id(
        channel_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT sys.id
                    FROM chat_systems AS sys
                    WHERE sys.events_channel = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&channel_id])
                    .map(move |row| ChatSystem {
                        id: row.get(0),
                        events_channel: channel_id,
                    })
                    .collect()
                    .map_err(lookup_error)
            })
            .and_then(|(mut systems, connection)| {
                if systems.len() > 0 {
                    Ok((systems.remove(0), connection))
                } else {
                    Err((EventErrorKind::Lookup.into(), connection))
                }
            })
    }

    /// Get a collection of every `ChatSystem` with its associated `Chats` from the database
    pub fn all_with_chats(
        connection: Connection,
    ) -> impl Future<Item = (Vec<(ChatSystem, Chat)>, Connection), Error = (EventError, Connection)>
    {
        let sql = "SELECT sys.id, sys.events_channel, ch.id, ch.chat_id
            FROM chats AS ch
            INNER JOIN chat_systems AS sys ON ch.system_id = sys.id";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[])
                    .map(|row| {
                        (
                            ChatSystem {
                                id: row.get(0),
                                events_channel: row.get(1),
                            },
                            Chat::from_parts(row.get(2), row.get(3)),
                        )
                    })
                    .collect()
                    .map_err(lookup_error)
            })
    }
}
