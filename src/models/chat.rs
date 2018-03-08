use std::collections::HashSet;

use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use super::chat_system::ChatSystem;
use super::event::Event;
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
    pub fn from_parts(id: i32, chat_id: Integer) -> Self {
        Chat { id, chat_id }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn chat_id(&self) -> Integer {
        self.chat_id
    }

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

    pub fn get_events(
        &self,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        Event::by_chat_id(self.chat_id, connection)
    }

    pub fn get_system_with_events(
        &self,
        connection: Connection,
    ) -> impl Future<
        Item = ((Option<ChatSystem>, HashSet<Chat>, Vec<Event>), Connection),
        Error = (EventError, Connection),
    > {
        ChatSystem::full_by_chat_id(self.chat_id, connection)
    }

    pub fn delete_by_id(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (u64, Connection), Error = (EventError, Connection)> {
        let sql = "DELETE FROM chats AS ch WHERE ch.id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| connection.execute(&s, &[&id]).map_err(delete_error))
    }

    pub fn delete(
        self,
        connection: Connection,
    ) -> impl Future<Item = (u64, Connection), Error = (EventError, Connection)> {
        Chat::delete_by_id(self.id, connection)
    }
}

pub struct CreateChat {
    pub chat_id: Integer,
}

impl CreateChat {
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
