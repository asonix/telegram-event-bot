#![feature(conservative_impl_trait)]

extern crate chrono;
extern crate futures;
extern crate futures_state_stream;
extern crate telebot;
extern crate time;
extern crate tokio_postgres;

use std::collections::HashMap;

use chrono::DateTime;
use chrono::offset::Utc;
use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::{Connection, Error};
use tokio_postgres::rows::Row;

/// ChatSystem represents a series of linked chats
///
/// `events_channel` is the ID of the channel where full announcements are made
/// `announce_chats` is as set of IDs where the bot should notify of announcements.
///
/// This is represented in the database as
///
/// ```
/// Relations:
/// chat_systems has_many chats (foreign_key on chats)
///
/// Columns:
/// id, events_channel
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatSystem {
    id: i32,
    events_channel: Integer,
    announce_chats: Vec<Integer>,
}

/*
#[sql("SELECT sys.id, sys.events_channel
        FROM chat_systems AS sys
        INNER JOIN chats AS ch ON ch.system_id = sys.id
        WHERE ch.chat_id = $1")]
pub struct LookupChatSystemByChatId {
    chat_id: Integer,
    chat_system: Option<ChatSystem>,
}
*/

/// Chat represents a single telegram chat
///
/// `chat_id` is the Telegram ID of the chat
///
/// ```
/// Relations:
/// chats belongs_to chat_systems (foreign_key on chats)
///
/// Columns:
/// id, system_id (foreign key), chat_id
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chat {
    id: i32,
    system_id: i32,
    chat_id: Integer,
}

/*
#[sql("SELECT ch.id, ch.chat_id
        FROM chats AS ch
        WHERE ch.system_id = $1")]
pub struct LookupChatBySystemId {
    system_id: i32,
    chat: Option<Chat>,
}

#[sql("SELECT ch.id, ch.system_id, ch.chat_id
        FROM chats AS ch
        INNER JOIN chat_systems AS sys ON ch.system_id = sys.id
        WHERE sys.channel_id = $1")]
pub struct LookupChatByChannelId {
    channel_id: i32,
    chat: Option<Chat>,
}
*/

/// Event represents a scheduled Event
///
/// `date` is the date of the event
/// `duration` represents how long the event is expected to last
/// `hosts` represents the user_ids of the users who are hosting the event
/// `title` is the name of the event
/// `description` is the description of the event
///
/// ```
/// Relations:
/// events belongs_to chat_systems (foreign_key on events)
/// events has_many hosts (foreign_key on hosts)
///
/// Columns:
/// id, system_id, date, duration, title, description
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    id: i32,
    date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    title: String,
    description: String,
    hosts: Vec<Integer>,
}

impl Event {
    fn condense_events_unordered(events: Vec<Self>) -> HashMap<i32, Self> {
        events.into_iter().fold(HashMap::new(), |mut acc, event| {
            let updated = {
                if let Some(mut stored_event) = acc.get_mut(&event.id) {
                    stored_event.hosts.extend(event.hosts.clone());
                    true
                } else {
                    false
                }
            };

            if !updated {
                acc.insert(event.id, event);
            }

            acc
        })
    }

    fn condense_events(events: Vec<Self>) -> Vec<Self> {
        events.into_iter().fold(Vec::new(), |mut acc, event| {
            let updated = {
                let len = acc.len();

                if let Some(mut prev_ev) = acc.get_mut(len - 1) {

                    if prev_ev.id == event.id {
                        prev_ev.hosts.extend(event.hosts.clone());
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if !updated {
                acc.push(event);
            }

            acc
        })
    }
}

pub fn lookup_event_full_by_chat_id_unordered(
    connection: Connection,
    chat_id: i32
) -> impl Future<Item = (HashMap<i32, Event>, Connection), Error = (Error, Connection)>
{
    let sql = "SELECT ev.id, ev.date, ev.end_date, ev.title, ev.description, h.id, h.user_id
               FROM events as ev
               INNER JOIN chat_systems AS sys ON ev.system_id = sys.id
               INNER JOIN chats AS ch ON ch.system_id = sys.id
               LEFT JOIN hosts AS h ON h.event_id = ev.id
               WHERE ch.id = $1";

    connection
        .prepare(sql)
        .and_then(move |(s, c)| {
            c.query(&s, &[&chat_id])
                .map(|row| {
                    // StateStream::map()
                    let host = Host::maybe_from_row(&row, 5, 6);

                    Event {
                        id: row.get(0),
                        date: row.get(1),
                        end_date: row.get(2),
                        title: row.get(3),
                        description: row.get(4),
                        hosts: host.into_iter().map(Host::into).collect(),
                    }
                })
                .collect()
                .map(|(events, c)| {
                    // Future::map()
                    (Event::condense_events_unordered(events), c)
                })
        })
}

pub fn lookup_event_full_by_chat_id(
    connection: Connection,
    chat_id: i32
) -> impl Future<Item = (Vec<Event>, Connection), Error = (Error, Connection)>
{
    let sql = "SELECT ev.id, ev.date, ev.end_date, ev.title, ev.description, h.id, h.user_id
               FROM events as ev
               INNER JOIN chat_systems AS sys ON ev.system_id = sys.id
               INNER JOIN chats AS ch ON ch.system_id = sys.id
               LEFT JOIN hosts AS h ON h.event_id = ev.id
               WHERE ch.id = $1
               ORDER BY ev.date, ev.id";

    connection
        .prepare(sql)
        .and_then(move |(s, c)| {
            c.query(&s, &[&chat_id])
                .map(|row| {
                    // StateStream::map()
                    let host = Host::maybe_from_row(&row, 5, 6);

                    Event {
                        id: row.get(0),
                        date: row.get(1),
                        end_date: row.get(2),
                        title: row.get(3),
                        description: row.get(4),
                        hosts: host.into_iter().map(Host::into).collect(),
                    }
                })
                .collect()
                .map(|(events, c)| {
                    // Future::map()
                    (Event::condense_events(events), c)
                })
        })
}

/*
#[sql("SELECT ev.id, ev.date, ev.end_date, ev.title, ev.description
        FROM events AS ev,
        INNER JOIN chat_systems AS sys ON ev.system_id = sys.id
        INNER JOIN chats AS ch ON ch.system_id = sys.id
        WHERE ch.id = $1")]
pub struct LookupEventByChatId {
    chat_id: Integer,
    event: Option<Event>,
}

#[sql("SELECT h.id, h.user_id, ev.id, ev.date, ev.end_date, ev.title, ev.description
        FROM events AS ev
        INNER JOIN chat_systems AS sys ON ev.system_id = sys.id
        INNER JOIN chats AS ch ON ch.system_id = sys.id
        LEFT JOIN hosts AS h ON h.event_id = ev.id
        WHERE ch.id = $1")]
pub struct LookupEventWithHostsByChatId {
    chat_id: i32,
    event_full: Option<Event>
}
*/

/// Host represents a host of a scheduled Event
///
/// `user_id` is the user_id of the host
///
/// ```
/// Relations:
/// hosts belongs_to events (foreign_key on hosts)
///
/// Columns:
/// id, event_id, user_id
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host {
    id: i32,
    user_id: Integer,
}

impl Host {
    fn maybe_from_row(row: &Row, id_index: usize, user_id_index: usize) -> Option<Host> {
        let id_opt: Option<i32> = row.get(id_index);
        let user_id_opt: Option<Integer> = row.get(user_id_index);

        id_opt.and_then(|id| {
            user_id_opt.map(|user_id| Host { id, user_id })
        })
    }

    /*
    fn from_row(row: &Row, id_index: usize, user_id_index: usize) -> Host {
        Host {
            id: row.get(id_index),
            user_id: row.get(user_id_index),
        }
    }
    */
}

impl From<Host> for Integer {
    fn from(host: Host) -> Self {
        host.user_id
    }
}

/*
#[sql("SELECT h.id, h.user_id
        FROM hosts AS h,
        INNER JOIN events AS ev
        WHERE h.event_id = ev.id")]
pub struct LookupHostByEventId {
    event_id: i32,
    host: Option<Host>,
}
*/
