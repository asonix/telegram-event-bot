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

use std::hash::{Hash, Hasher};

use chrono::offset::Utc;
use chrono::DateTime;
use chrono_tz::Tz;
use failure::ResultExt;
use futures::{Future, IntoFuture};
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::stmt::Statement;
use tokio_postgres::transaction::Transaction;
use tokio_postgres::types::ToSql;
use tokio_postgres::Connection;

use super::user::User;
use error::{EventError, EventErrorKind};
use util::*;

/// Event represents a scheduled Event
///
/// `start_date` is the date of the event
/// `end_date` is the date the event ends
/// `hosts` represents the user_ids of the users who are hosting the event
/// `title` is the name of the event
/// `description` is the description of the event
///
/// ### Relations:
/// - events belongs_to chat_systems (foreign_key on events)
/// - events has_many hosts (foreign_key on hosts)
///
/// ### Columns:
/// - id SERIAL
/// - start_date TIMESTAMP WITH TIME ZONE
/// - end_date TIMESTAMP WITH TIME ZONE
/// - title TEXT
/// - description TEXT
/// - system_id INTEGER REFERENCES chat_systems
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    id: i32,
    start_date: DateTime<Tz>,
    end_date: DateTime<Tz>,
    title: String,
    description: String,
    hosts: Vec<User>,
    system_id: i32,
}

impl Hash for Event {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Event {
    /// Get the `Event` database ID
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the `Event` start date
    pub fn start_date(&self) -> &DateTime<Tz> {
        &self.start_date
    }

    /// Get the `Event` end date
    pub fn end_date(&self) -> &DateTime<Tz> {
        &self.end_date
    }

    /// Get the `Event` title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the `Event` description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the Users hosting the `Event`
    pub fn hosts(&self) -> &[User] {
        self.hosts.as_slice()
    }

    /// Get the ID of the associated `ChatSystem`
    pub fn system_id(&self) -> i32 {
        self.system_id
    }

    /// Merge two events that are the same, appending hosts but overwriting other fields, puttign
    /// the result on the end of a vector
    pub fn condense(events: &mut Vec<Self>, mut event_1: Self, event_2: Self) {
        let these_events = if event_1.id != event_2.id {
            vec![event_1, event_2]
        } else {
            event_1.hosts.extend(event_2.hosts.clone());
            vec![event_1]
        };

        events.extend(these_events);
    }

    /// Merge events that are the same, appending hosts but overwriting other fields
    fn condense_events(events: Vec<Self>) -> Vec<Self> {
        events.into_iter().fold(Vec::new(), |mut acc, event| {
            let len = acc.len();

            if len > 0 {
                let prev_ev = acc.remove(len - 1);

                Event::condense(&mut acc, prev_ev, event);
            } else {
                acc.push(event);
            }

            acc
        })
    }

    /// Lookup event by the host's id
    pub fn by_user_id(
        user_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT evt.id, evt.system_id, evt.start_date, evt.end_date, evt.title, evt.description, evt.timezone, usr.id, usr.user_id, usr.username
                    FROM events AS evt
                    LEFT JOIN hosts AS h ON h.events_id = evt.id
                    INNER JOIN users AS usr ON usr.id = h.users_id
                    WHERE usr.user_id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&user_id])
                    .map(move |row| {
                        let tz: String = row.get(6);

                        let sd: DateTime<Utc> = row.get(2);
                        let ed: DateTime<Utc> = row.get(3);

                        tz.parse::<Tz>().map(|timezone| Event {
                            id: row.get(0),
                            start_date: sd.with_timezone(&timezone),
                            end_date: ed.with_timezone(&timezone),
                            title: row.get(4),
                            description: row.get(5),
                            hosts: User::maybe_from_parts(row.get(7), row.get(8), row.get(9))
                                .into_iter()
                                .collect(),
                            system_id: row.get(1),
                        })
                    })
                    .collect()
                    .map_err(lookup_error)
            })
            .map(|(events, connection)| {
                (
                    Event::condense_events(events.into_iter().filter_map(Result::ok).collect()),
                    connection,
                )
            })
    }

    /// Lookup event by the event id
    pub fn by_id(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (Event, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT evt.system_id, evt.start_date, evt.end_date, evt.title, evt.description, evt.timezone, usr.id, usr.user_id, usr.username
                    FROM events AS evt
                    LEFT JOIN hosts AS h ON h.events_id = evt.id
                    INNER JOIN users AS usr ON usr.id = h.users_id
                    WHERE evt.id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&id])
                    .map(move |row| {
                        let tz: String = row.get(5);

                        let sd: DateTime<Utc> = row.get(1);
                        let ed: DateTime<Utc> = row.get(2);

                        tz.parse::<Tz>().map(|timezone| Event {
                            id,
                            start_date: sd.with_timezone(&timezone),
                            end_date: ed.with_timezone(&timezone),
                            title: row.get(3),
                            description: row.get(4),
                            hosts: User::maybe_from_parts(row.get(6), row.get(7), row.get(8))
                                .into_iter()
                                .collect(),
                            system_id: row.get(0),
                        })
                    })
                    .collect()
                    .map_err(lookup_error)
            })
            .and_then(|(mut events, connection)| {
                if events.len() > 0 {
                    if let Ok(event) = events.remove(0) {
                        Ok((event, connection))
                    } else {
                        Err((EventErrorKind::Lookup.into(), connection))
                    }
                } else {
                    Err((EventErrorKind::Lookup.into(), connection))
                }
            })
    }

    /// Delete and `Event` and all associated `hosts` given an ID
    pub fn delete_by_id(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (u64, Connection), Error = (EventError, Connection)> {
        let sql = "DELETE FROM events AS ev WHERE ev.id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| connection.execute(&s, &[&id]).map_err(delete_error))
    }

    /// Get a `Vec<Event>` with events happening within the next `start_date` to `end_date`
    pub fn in_range(
        start_date: DateTime<Tz>,
        end_date: DateTime<Tz>,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT DISTINCT ev.id, ev.start_date, ev.end_date, ev.title, ev.description, ev.system_id, ev.timezone
                    FROM events AS ev
                    WHERE ev.start_date > $1 AND ev.start_date < $2";
        debug!("{}", sql);

        let sd = start_date.with_timezone(&Utc);
        let ed = end_date.with_timezone(&Utc);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&sd, &ed])
                    .map(|row| {
                        let sd: DateTime<Utc> = row.get(1);
                        let ed: DateTime<Utc> = row.get(2);

                        let tz: String = row.get(6);

                        tz.parse::<Tz>().map(|timezone| Event {
                            id: row.get(0),
                            start_date: sd.with_timezone(&timezone),
                            end_date: ed.with_timezone(&timezone),
                            title: row.get(3),
                            description: row.get(4),
                            hosts: Vec::new(),
                            system_id: row.get(5),
                        })
                    })
                    .collect()
                    .map(|(events, connection)| {
                        (
                            events.into_iter().filter_map(Result::ok).collect(),
                            connection,
                        )
                    })
                    .map_err(lookup_error)
            })
    }

    /// Given the system id, lookup all associated events
    ///
    /// This creates a future whose item contains the database connection and an ordered vector of
    /// event structs. The events are ordered date.
    pub fn by_system_id(
        system_id: i32,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Self>, Connection), Error = (EventError, Connection)> {
        let sql =
            "SELECT evt.id, evt.start_date, evt.end_date, evt.title, evt.description, evt.timezone, usr.id, usr.user_id, usr.username
                FROM events AS evt
                LEFT JOIN hosts AS h ON h.events_id = evt.id
                INNER JOIN users AS usr ON usr.id = h.users_id
                WHERE evt.system_id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&system_id])
                    .map(move |row| {
                        let tz: String = row.get(5);

                        let sd: DateTime<Utc> = row.get(1);
                        let ed: DateTime<Utc> = row.get(2);

                        tz.parse::<Tz>().map(|timezone| Event {
                            id: row.get(0),
                            start_date: sd.with_timezone(&timezone),
                            end_date: ed.with_timezone(&timezone),
                            title: row.get(3),
                            description: row.get(4),
                            hosts: User::maybe_from_parts(row.get(6), row.get(7), row.get(8))
                                .into_iter()
                                .collect(),
                            system_id: system_id,
                        })
                    })
                    .collect()
                    .map_err(lookup_error)
                    .map(|(events, connection)| {
                        (
                            Event::condense_events(
                                events.into_iter().filter_map(Result::ok).collect(),
                            ),
                            connection,
                        )
                    })
            })
    }

    /// Given a chat id, lookup all associated events
    ///
    /// This creates a future whose item contains the database connection and an ordered vector of
    /// event structs. The events are ordered date.
    pub fn by_chat_id(
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Self>, Connection), Error = (EventError, Connection)> {
        let sql =
            "SELECT evt.id, evt.start_date, evt.end_date, evt.title, evt.description, evt.timezone, usr.id, usr.user_id, usr.username, sys.id
               FROM events as evt
               INNER JOIN chat_systems AS sys ON evt.system_id = sys.id
               INNER JOIN chats AS ch ON ch.system_id = sys.id
               LEFT JOIN hosts AS h ON h.events_id = evt.id
               LEFT JOIN users AS usr ON h.users_id = usr.id
               WHERE ch.chat_id = $1
               ORDER BY evt.start_date, evt.id";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[&chat_id])
                    .map(|row| {
                        // StateStream::map()
                        let host = User::maybe_from_parts(row.get(6), row.get(7), row.get(8));
                        let tz: String = row.get(5);

                        let sd: DateTime<Utc> = row.get(1);
                        let ed: DateTime<Utc> = row.get(2);

                        tz.parse::<Tz>().map(|timezone| Event {
                            id: row.get(0),
                            start_date: sd.with_timezone(&timezone),
                            end_date: ed.with_timezone(&timezone),
                            title: row.get(3),
                            description: row.get(4),
                            hosts: host.into_iter().collect(),
                            system_id: row.get(9),
                        })
                    })
                    .collect()
                    .map(|(events, connection)| {
                        // Future::map()
                        (
                            Event::condense_events(
                                events.into_iter().filter_map(Result::ok).collect(),
                            ),
                            connection,
                        )
                    })
                    .map_err(lookup_error)
            })
    }
}

/// This type exists as a way to safely update events in the database.
///
/// If all fields are provided and an UpdateEvent is successfully created, the event can be safely
/// updated in the database.
#[derive(Clone, Debug)]
pub struct UpdateEvent {
    pub id: i32,
    pub system_id: i32,
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
    pub title: String,
    pub description: String,
    pub hosts: Vec<i32>,
}

impl UpdateEvent {
    /// Perform the database interaction to update the event
    pub fn update(
        self,
        connection: Connection,
    ) -> impl Future<Item = (Event, Connection), Error = (EventError, Connection)> {
        let sql = "UPDATE events
                    SET start_date = $1, end_date = $2, title = $3, description = $4, timezone = $5
                    WHERE id = $6";
        debug!("{}", sql);

        let UpdateEvent {
            id,
            system_id,
            start_date,
            end_date,
            title,
            description,
            hosts: _hosts,
        } = self;

        let timezone = start_date.timezone().name();
        let sd = start_date.with_timezone(&Utc);
        let ed = end_date.with_timezone(&Utc);

        connection
            .prepare(&sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .execute(&s, &[&sd, &ed, &title, &description, &timezone, &id])
                    .map_err(update_error)
                    .and_then(move |(count, connection)| {
                        if count > 0 {
                            Ok((
                                Event {
                                    id,
                                    system_id,
                                    start_date,
                                    end_date,
                                    title,
                                    description,
                                    hosts: Vec::new(),
                                },
                                connection,
                            ))
                        } else {
                            Err((EventErrorKind::Update.into(), connection))
                        }
                    })
            })
    }
}

/// This type provides a safe way to create events in the database
#[derive(Clone, Debug)]
pub struct CreateEvent {
    pub system_id: i32,
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
    pub title: String,
    pub description: String,
    pub hosts: Vec<User>,
}

impl CreateEvent {
    /// Create a future which yields the new Event
    pub fn create(
        self,
        connection: Connection,
    ) -> impl Future<Item = (Event, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO events (start_date, end_date, title, description, system_id, timezone) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id";
        debug!("{}", sql);

        let CreateEvent {
            system_id,
            start_date,
            end_date,
            title,
            description,
            hosts,
        } = self;

        connection
            .transaction()
            .map_err(transaction_error)
            .and_then(move |transaction| {
                insert_event(
                    sql,
                    system_id,
                    start_date,
                    end_date,
                    title,
                    description,
                    hosts,
                    transaction,
                ).or_else(|(e, transaction)| {
                    transaction
                        .rollback()
                        .or_else(|(_, connection)| Err(connection))
                        .then(move |res| match res {
                            Ok(connection) => Err((e, connection)),
                            Err(connection) => Err((e, connection)),
                        })
                })
                    .and_then(|(event, transaction)| {
                        transaction
                            .commit()
                            .map_err(commit_error)
                            .map(move |connection| (event, connection))
                    })
            })
    }
}

fn insert_event(
    sql: &str,
    id: i32,
    start_date: DateTime<Tz>,
    end_date: DateTime<Tz>,
    title: String,
    description: String,
    hosts: Vec<User>,
    transaction: Transaction,
) -> impl Future<Item = (Event, Transaction), Error = (EventError, Transaction)> {
    let sd = start_date.with_timezone(&Utc);
    let ed = end_date.with_timezone(&Utc);
    transaction
        .prepare(sql)
        .map_err(transaction_prepare_error)
        .and_then(move |(s, transaction)| {
            transaction
                .query(
                    &s,
                    &[
                        &sd,
                        &ed,
                        &title,
                        &description,
                        &id,
                        &start_date.timezone().name(),
                    ],
                )
                .map(move |row| Event {
                    id: row.get(0),
                    start_date: start_date,
                    end_date: end_date,
                    title: title.clone(),
                    description: description.clone(),
                    hosts: Vec::new(),
                    system_id: id,
                })
                .collect()
                .map_err(transaction_insert_error)
                .and_then(|(mut events, transaction)| {
                    if events.len() > 0 {
                        Ok((events.remove(0), transaction))
                    } else {
                        Err((EventErrorKind::Insert.into(), transaction))
                    }
                })
                .and_then(move |(event, transaction)| insert_hosts(hosts, event, transaction))
        })
}

fn prepare_hosts(
    hosts: &[User],
    event: Event,
    transaction: Transaction,
) -> Result<(String, Event, Transaction), (EventError, Event, Transaction)> {
    if hosts.len() > 0 {
        let sql = "INSERT INTO hosts (users_id, events_id) VALUES".to_owned();
        debug!("{}", sql);

        let values = hosts
            .iter()
            .fold((Vec::new(), 1), |(mut acc, count), _| {
                acc.push(format!("(${}, ${})", count, count + 1));

                (acc, count + 2)
            })
            .0
            .join(", ");

        Ok((
            format!("{} {} RETURNING users_id", sql, values),
            event,
            transaction,
        ))
    } else {
        Err((EventErrorKind::Hosts.into(), event, transaction))
    }
}

fn insert_hosts(
    hosts: Vec<User>,
    event: Event,
    transaction: Transaction,
) -> impl Future<Item = (Event, Transaction), Error = (EventError, Transaction)> {
    prepare_hosts(&hosts, event, transaction)
        .into_future()
        .and_then(move |(hosts_sql, event, transaction)| {
            insert_hosts_prepare(hosts, hosts_sql, event, transaction)
        })
        .or_else(
            move |(e, event, transaction): (EventError, _, Transaction)| {
                if *e.context.get_context() == EventErrorKind::Hosts {
                    Ok((event, transaction))
                } else {
                    Err((e, transaction))
                }
            },
        )
}

fn insert_hosts_prepare(
    hosts: Vec<User>,
    hosts_sql: String,
    event: Event,
    transaction: Transaction,
) -> impl Future<Item = (Event, Transaction), Error = (EventError, Event, Transaction)> {
    transaction
        .prepare(&hosts_sql)
        .then(move |res| match res {
            Ok((s, transaction)) => Ok((s, event, transaction)),
            Err((e, transaction)) => Err((e, event, transaction)),
        })
        .or_else(|(e, event, transaction)| {
            Err(e)
                .context(EventErrorKind::Prepare)
                .map_err(|e| (e.into(), event, transaction))
        })
        .and_then(move |(statement, event, transaction)| {
            insert_hosts_query(hosts, statement, event, transaction)
        })
}

fn insert_hosts_query(
    hosts: Vec<User>,
    statement: Statement,
    mut event: Event,
    transaction: Transaction,
) -> impl Future<Item = (Event, Transaction), Error = (EventError, Event, Transaction)> {
    let id = event.id();

    let host_ids: Vec<_> = hosts.iter().map(|user| user.id()).collect();

    let host_args = host_ids.iter().fold(Vec::new(), |mut acc, user_id| {
        acc.push(user_id as &ToSql);
        acc.push(&id as &ToSql);
        acc
    });

    let num_hosts = hosts.len();

    transaction
        .query(&statement, host_args.as_slice())
        .map(move |row| row.get(0))
        .collect()
        .map_err(transaction_insert_error)
        .and_then(move |(users_ids, transaction): (Vec<i32>, _)| {
            if users_ids.len() == num_hosts {
                Ok((hosts, transaction))
            } else {
                Err((EventErrorKind::Insert.into(), transaction))
            }
        })
        .then(|res| match res {
            Ok((hosts, transaction)) => {
                event.hosts.extend(hosts);

                Ok((event, transaction))
            }
            Err((e, transaction)) => Err((e, event, transaction)),
        })
}
