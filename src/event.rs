use std::collections::HashMap;

use chrono::DateTime;
use chrono::offset::Utc;
use failure::ResultExt;
use futures::{Future, IntoFuture};
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;
use tokio_postgres::stmt::Statement;
use tokio_postgres::transaction::Transaction;
use tokio_postgres::types::ToSql;

use chat_system::ChatSystem;
use error::{EventError, EventErrorKind};
use host::Host;
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
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    title: String,
    description: String,
    hosts: Vec<Host>,
}

impl Event {
    pub fn from_parts(
        id: i32,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        title: String,
        description: String,
    ) -> Self {
        let hosts = Vec::new();

        Event {
            id,
            start_date,
            end_date,
            title,
            description,
            hosts,
        }
    }

    pub fn add_host(&mut self, host: Option<Host>) {
        self.hosts.extend(host);
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn start_date(&self) -> &DateTime<Utc> {
        &self.start_date
    }

    pub fn end_date(&self) -> &DateTime<Utc> {
        &self.end_date
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn hosts(&self) -> &[Host] {
        self.hosts.as_slice()
    }

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

    pub fn condense(events: &mut Vec<Self>, mut event_1: Self, event_2: Self) {
        let these_events = if event_1.id != event_2.id {
            vec![event_1, event_2]
        } else {
            event_1.hosts.extend(event_2.hosts.clone());
            vec![event_1]
        };

        events.extend(these_events);
    }

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

    /// Delete an `Event` and all associated `Hosts`
    pub fn delete(
        self,
        connection: Connection,
    ) -> Box<Future<Item = (u64, Connection), Error = (EventError, Connection)>> {
        let sql = "DELETE FROM events WHERE id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection.execute(&s, &[&self.id]).map_err(delete_error)
                }),
        )
    }

    /// Given a chat id, lookup all associated events
    ///
    /// This event list is unordered, which improves lookup time, but may be slower if the end result
    /// must be provided in order of date
    pub fn by_chat_id_unordered(
        chat_id: i32,
        connection: Connection,
    ) -> Box<Future<Item = (HashMap<i32, Self>, Connection), Error = (EventError, Connection)>>
    {
        let sql =
            "SELECT evt.id, evt.start_date, evt.end_date, evt.title, evt.description, h.id, h.user_id
               FROM events as evt
               INNER JOIN chat_systems AS sys ON evt.system_id = sys.id
               INNER JOIN chats AS ch ON ch.system_id = sys.id
               LEFT JOIN hosts AS h ON h.event_id = evt.id
               WHERE ch.id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection
                        .query(&s, &[&chat_id])
                        .map(|row| {
                            // StateStream::map()
                            let host = Host::maybe_from_row(&row, 5, 6);

                            Event {
                                id: row.get(0),
                                start_date: row.get(1),
                                end_date: row.get(2),
                                title: row.get(3),
                                description: row.get(4),
                                hosts: host.into_iter().map(Host::into).collect(),
                            }
                        })
                        .collect()
                        .map(|(events, connection)| {
                            // Future::map()
                            (Event::condense_events_unordered(events), connection)
                        })
                        .map_err(lookup_error)
                }),
        )
    }

    /// Given a chat id, lookup all associated events
    ///
    /// This creates a future whose item contains the database connection and an ordered vector of
    /// event structs. The events are ordered date.
    pub fn by_chat_id(
        chat_id: i32,
        connection: Connection,
    ) -> Box<Future<Item = (Vec<Self>, Connection), Error = (EventError, Connection)>> {
        let sql =
            "SELECT evt.id, evt.start_date, evt.end_date, evt.title, evt.description, h.id, h.user_id
               FROM events as evt
               INNER JOIN chat_systems AS sys ON evt.system_id = sys.id
               INNER JOIN chats AS ch ON ch.system_id = sys.id
               LEFT JOIN hosts AS h ON h.event_id = evt.id
               WHERE ch.id = $1
               ORDER BY evt.start_date, evt.id";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection
                        .query(&s, &[&chat_id])
                        .map(|row| {
                            // StateStream::map()
                            let host = Host::maybe_from_row(&row, 5, 6);

                            Event {
                                id: row.get(0),
                                start_date: row.get(1),
                                end_date: row.get(2),
                                title: row.get(3),
                                description: row.get(4),
                                hosts: host.into_iter().map(Host::into).collect(),
                            }
                        })
                        .collect()
                        .map(|(events, connection)| {
                            // Future::map()
                            (Event::condense_events(events), connection)
                        })
                        .map_err(lookup_error)
                }),
        )
    }
}

#[derive(Clone, Debug)]
pub struct CreateEvent {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub hosts: Vec<Integer>,
}

impl CreateEvent {
    /// Create a future which yields the new Event
    pub fn create(
        self,
        chat_system: &ChatSystem,
        connection: Connection,
    ) -> Box<Future<Item = (Event, Connection), Error = (EventError, Connection)>> {
        let sql = "INSERT INTO events (start_date, end_date, title, description) VALUES ($1, $2, $3, $4) WHERE system_id = $5 RETURNING id";

        let CreateEvent {
            start_date,
            end_date,
            title,
            description,
            hosts,
        } = self;

        let id = chat_system.id();

        Box::new(
            connection
                .transaction()
                .map_err(transaction_error)
                .and_then(move |transaction| {
                    insert_event(
                        sql,
                        id,
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
                }),
        )
    }
}

fn insert_event(
    sql: &str,
    id: i32,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    title: String,
    description: String,
    hosts: Vec<Integer>,
    transaction: Transaction,
) -> Box<Future<Item = (Event, Transaction), Error = (EventError, Transaction)>> {
    Box::new(
        transaction
            .prepare(sql)
            .map_err(transaction_prepare_error)
            .and_then(move |(s, transaction)| {
                transaction
                    .query(&s, &[&start_date, &end_date, &title, &description, &id])
                    .map(move |row| Event {
                        id: row.get(0),
                        start_date: start_date,
                        end_date: end_date,
                        title: title.clone(),
                        description: description.clone(),
                        hosts: Vec::new(),
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
            }),
    )
}

fn prepare_hosts(
    hosts: &[Integer],
    event: Event,
    transaction: Transaction,
) -> Result<(String, Event, Transaction), (EventError, Event, Transaction)> {
    if hosts.len() > 0 {
        let sql = "INSERT INTO hosts (user_id, events_id) VALUES".to_owned();

        let values = hosts
            .iter()
            .fold((Vec::new(), 0), |(mut acc, count), _| {
                acc.push(format!("({}, {})", count, count + 1));

                (acc, count + 2)
            })
            .0
            .join(", ");

        Ok((
            format!("{} {} RETURNING id, user_id", sql, values),
            event,
            transaction,
        ))
    } else {
        Err((EventErrorKind::Hosts.into(), event, transaction))
    }
}

fn insert_hosts(
    hosts: Vec<Integer>,
    event: Event,
    transaction: Transaction,
) -> Box<Future<Item = (Event, Transaction), Error = (EventError, Transaction)>> {
    Box::new(
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
            ),
    )
}

fn insert_hosts_prepare(
    hosts: Vec<Integer>,
    hosts_sql: String,
    event: Event,
    transaction: Transaction,
) -> Box<Future<Item = (Event, Transaction), Error = (EventError, Event, Transaction)>> {
    Box::new(
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
                inser_hosts_query(hosts, statement, event, transaction)
            }),
    )
}

fn inser_hosts_query(
    hosts: Vec<Integer>,
    statement: Statement,
    mut event: Event,
    transaction: Transaction,
) -> Box<Future<Item = (Event, Transaction), Error = (EventError, Event, Transaction)>> {
    let hosts_refs: Vec<_> = hosts.iter().map(|user_id| user_id as &ToSql).collect();

    let hosts_clone = hosts.clone();

    Box::new(
        transaction
            .query(&statement, hosts_refs.as_slice())
            .map(move |row| Host::from_row(&row, 0, 1))
            .collect()
            .map_err(transaction_insert_error)
            .and_then(move |(returned_hosts, transaction)| {
                if returned_hosts.len() == hosts_clone.len() {
                    Ok((returned_hosts, transaction))
                } else {
                    Err((EventErrorKind::Insert.into(), transaction))
                }
            })
            .then(|res| match res {
                Ok((returned_hosts, transaction)) => {
                    event.hosts.extend(returned_hosts);

                    Ok((event, transaction))
                }
                Err((e, transaction)) => Err((e, event, transaction)),
            }),
    )
}
