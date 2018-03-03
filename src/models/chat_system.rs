use std::collections::HashSet;

use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;
use tokio_postgres::rows::Row;

use error::{EventError, EventErrorKind};
use super::chat::Chat;
use super::event::Event;
use super::user::User;
use super::util::*;

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
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn events_channel(&self) -> Integer {
        self.events_channel
    }

    pub fn create(
        events_channel: Integer,
        connection: Connection,
    ) -> Box<Future<Item = (Self, Connection), Error = (EventError, Connection)>> {
        let sql = "INSERT INTO chat_systems (events_channel) VALUES ($1) RETURNING id";

        Box::new(
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
                }),
        )
    }

    /// Fetch a chat system given it's ID
    pub fn by_id(
        id: i32,
        connection: Connection,
    ) -> Box<Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)>> {
        let sql = "SELECT sys.id, sys.events_channel
                    FROM chat_systems AS sys
                    WHERE sys.id = $1";

        Box::new(
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
                }),
        )
    }

    /// Delete a `ChatSystem` and all associated `Chats`, `Events`, and `Users` given an id
    pub fn delete_by_id(
        id: i32,
        connection: Connection,
    ) -> Box<Future<Item = (u64, Connection), Error = (EventError, Connection)>> {
        let sql = "DELETE FROM chat_systems AS sys WHERE sys.id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection.execute(&s, &[&id]).map_err(delete_error)
                }),
        )
    }

    /// Delete a `ChatSystem` and all associated `Chats`, `Events`, and `Users`
    pub fn delete(
        self,
        connection: Connection,
    ) -> Box<Future<Item = (u64, Connection), Error = (EventError, Connection)>> {
        ChatSystem::delete_by_id(self.id, connection)
    }

    /// Select the chat system by event id
    pub fn by_event_id(
        event_id: i32,
        connection: Connection,
    ) -> Box<Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)>> {
        let sql = "SELECT sys.id, sys.events_channel
                    FROM chat_systems AS sys
                    LEFT JOIN events AS ev ON ev.system_id = sys.id
                    WHERE ev.id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection
                        .query(&s, &[&event_id])
                        .map(|row| ChatSystem {
                            id: row.get(0),
                            events_channel: row.get(1),
                        })
                        .collect()
                        .map_err(lookup_error)
                })
                .and_then(|(mut chat_systems, connection)| {
                    if chat_systems.len() == 1 {
                        Ok((chat_systems.remove(0), connection))
                    } else {
                        Err((EventErrorKind::Lookup.into(), connection))
                    }
                }),
        )
    }

    /// Select the chat system by channel id
    pub fn by_channel_id(
        channel_id: Integer,
        connection: Connection,
    ) -> Box<Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)>> {
        let sql = "SELECT sys.id
                    FROM chat_systems AS sys
                    WHERE sys.events_channel = $1";

        Box::new(
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
                }),
        )
    }

    /// Select all chat systems a user belongs to
    pub fn by_user_id(
        user_id: Integer,
        connection: Connection,
    ) -> Box<Future<Item = (Vec<ChatSystem>, Connection), Error = (EventError, Connection)>> {
        let sql = "SELECT
                    DISTINCT sys.id, sys.events_channel
                    FROM chat_systems AS sys
                    INNER JOIN chats AS ch ON ch.system_id = sys.id
                    INNER JOIN user_chats AS usch ON usch.chats_id = ch.id
                    INNER JOIN users AS usr ON usch.users_id = usr.id
                    WHERE usr.user_id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection
                        .query(&s, &[&user_id])
                        .map(|row| ChatSystem {
                            id: row.get(0),
                            events_channel: row.get(1),
                        })
                        .collect()
                        .map_err(lookup_error)
                }),
        )
    }

    pub fn all_with_chats(
        connection: Connection,
    ) -> impl Future<Item = (Vec<(ChatSystem, Chat)>, Connection), Error = (EventError, Connection)>
    {
        let sql = "SELECT sys.id, sys.events_channel, ch.id, ch.chat_id
            FROM chats AS ch
            INNER JOIN chat_systems AS sys ON ch.system_id = sys.id";

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

    /// Select an `Option<ChatSystem>`, `HashSet<Chat>`, and ordered `Vec<Event>` given a `chat_id`
    pub fn full_by_chat_id(
        chat_id: Integer,
        connection: Connection,
    ) -> Box<
        Future<
            Item = ((Option<Self>, HashSet<Chat>, Vec<Event>), Connection),
            Error = (EventError, Connection),
        >,
    > {
        let sql = "SELECT
                        sys.id, sys.events_channel,
                        ch.id, ch.chat_id,
                        ev.id, ev.start_date, ev.end_date, ev.title, ev.description,
                        usr.id, usr.user_id
                    FROM chats AS ch
                    INNER JOIN chat_systems AS sys ON ch.system_id = sys.id
                    LEFT JOIN events AS ev ON ev.system_id = sys.id
                    LEFT JOIN hosts AS h ON h.events_id = ev.id
                    LEFT JOIN users AS usr ON h.users_id = usr.id
                    WHERE ch.chat_id = $1
                    ORDER BY ev.start_date, ev.id";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection
                        .query(&s, &[&chat_id])
                        .map(|row: Row| -> (Self, Chat, Option<Event>) {
                            let sys = ChatSystem {
                                id: row.get(0),
                                events_channel: row.get(1),
                            };

                            let ch = Chat::from_parts(row.get(2), row.get(3));

                            let ev = Event::maybe_from_parts(
                                row.get(4),
                                row.get(5),
                                row.get(6),
                                row.get(7),
                                row.get(8),
                                Some(row.get(0)),
                            ).map(|mut ev| {
                                let host = User::maybe_from_parts(row.get(9), row.get(10));

                                ev.add_host(host);
                                ev
                            });

                            (sys, ch, ev)
                        })
                        .collect()
                        .map(|(rows, connection)| {
                            let vals = rows.into_iter().fold(
                                (None, HashSet::new(), Vec::new()),
                                |(_, mut chat_hash, mut event_vec), (sys, ch, ev)| {
                                    let len = event_vec.len();

                                    if len > 0 {
                                        let prev_event = event_vec.remove(len - 1);
                                        if let Some(ev) = ev {
                                            Event::condense(&mut event_vec, prev_event, ev);
                                        } else {
                                            event_vec.push(prev_event);
                                        }
                                    } else {
                                        if let Some(ev) = ev {
                                            event_vec.push(ev);
                                        }
                                    }

                                    chat_hash.insert(ch);

                                    (Some(sys), chat_hash, event_vec)
                                },
                            );

                            (vals, connection)
                        })
                        .map_err(lookup_error)
                }),
        )
    }
}
