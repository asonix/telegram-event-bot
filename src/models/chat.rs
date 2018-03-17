//! This module defines the `Chat` struct, and associated types and functions.

use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use super::chat_system::ChatSystem;
use util::*;

/// Chat represents a single telegram chat
///
/// `chat_id` is the Telegram ID of the chat
///
/// Relations:
/// chats belongs_to chat_systems (foreign_key on chats)
///
/// Columns:
/// - id SERIAL
/// - chat_id BIGINT
/// - system_id INTEGER REFERENCES chat_systems
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Chat {
    id: i32,
    chat_id: Integer,
}

impl Chat {
    /// Create a `Chat` from the parts that make up a `Chat`
    pub fn from_parts(id: i32, chat_id: Integer) -> Self {
        Chat { id, chat_id }
    }

    /// Get the chat's ID
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the chat's Telegram ID
    pub fn chat_id(&self) -> Integer {
        self.chat_id
    }

    /// Get a chat from the database given the chat's Telegram ID
    pub fn by_chat_id(
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Chat, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT id FROM chats AS ch WHERE ch.chat_id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&chat_id])
                    .map(move |row| Chat {
                        id: row.get(0),
                        chat_id: chat_id,
                    })
                    .collect()
                    .map_err(lookup_error)
            })
            .and_then(|(mut chats, connection)| {
                if chats.len() > 0 {
                    Ok((chats.remove(0), connection))
                } else {
                    Err((EventErrorKind::Lookup.into(), connection))
                }
            })
    }
}

/// This struct is used when inserting chats into the database
///
/// Since a chat is only made up of an ID and a Chat ID, only the Chat ID is required to insert a
/// `Chat`.
pub struct CreateChat {
    /// The Telegram ID of the chat to be inserted
    pub chat_id: Integer,
}

impl CreateChat {
    /// Insert the `CreateChat` into the `chats` table, returning the created `Chat` or an
    /// `EventError`
    pub fn create(
        self,
        chat_system: &ChatSystem,
        connection: Connection,
    ) -> impl Future<Item = (Chat, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO chats (chat_id, system_id) VALUES ($1, $2) RETURNING id";
        debug!("{}", sql);

        let chat_id = self.chat_id;
        let system_id = chat_system.id();

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&chat_id, &system_id])
                    .map(move |row| Chat {
                        id: row.get(0),
                        chat_id: chat_id,
                    })
                    .collect()
                    .map_err(insert_error)
                    .and_then(|(mut chats, connection)| {
                        if chats.len() > 0 {
                            Ok((chats.remove(0), connection))
                        } else {
                            Err((EventErrorKind::Insert.into(), connection))
                        }
                    })
            })
    }
}
