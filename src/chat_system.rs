use std::collections::HashSet;

use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;
use tokio_postgres::rows::Row;

use chat::Chat;
use error::{EventError, EventErrorKind};
use event::Event;
use host::Host;
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

    /// Delete a `ChatSystem` and all associated `Chats`, `Events`, and `Hosts`
    pub fn delete(
        self,
        connection: Connection,
    ) -> Box<Future<Item = (u64, Connection), Error = (EventError, Connection)>> {
        let sql = "DELETE FROM chat_systems AS sys WHERE sys.id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection.execute(&s, &[&self.id]).map_err(delete_error)
                }),
        )
    }

    /// Select an `Option<ChatSystem>`, `HashSet<Chat>`, and ordered `Vec<Event>` given a `chat_id`
    pub fn by_chat_id(
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<
        Item = ((Option<Self>, HashSet<Chat>, Vec<Event>), Connection),
        Error = (EventError, Connection),
    > {
        let sql = "SELECT
                        sys.id, sys.events_channel,
                        ch.id, ch.chat_id,
                        ev.id, ev.start_date, ev.end_date, ev.title, ev.description,
                        h.id, h.user_id
                    FROM chats AS ch
                    INNER JOIN chat_systems AS sys ON ch.system_id = sys.id
                    LEFT JOIN events AS ev ON ev.system_id = sys.id
                    LEFT JOIN hosts AS h ON h.event_id = ev.id
                    WHERE ch.chat_id = $1
                    ORDER BY ev.date, ev.id";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection
                        .query(&s, &[&chat_id])
                        .map(|row: Row| -> (Self, Chat, Event) {
                            let sys = ChatSystem {
                                id: row.get(0),
                                events_channel: row.get(1),
                            };

                            let ch = Chat::from_parts(row.get(2), row.get(3));

                            let mut ev = Event::from_parts(
                                row.get(4),
                                row.get(5),
                                row.get(6),
                                row.get(7),
                                row.get(8),
                            );

                            let host = Host::maybe_from_row(&row, 9, 10);

                            ev.add_host(host);

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
                                        Event::condense(&mut event_vec, prev_event, ev);
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
