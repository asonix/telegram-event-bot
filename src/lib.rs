#![feature(conservative_impl_trait)]
#![type_length_limit = "2097152"]

extern crate chrono;
extern crate dotenv;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate futures;
extern crate futures_state_stream;
extern crate telebot;
extern crate time;
extern crate tokio_core;
extern crate tokio_postgres;

pub mod chat;
pub mod chat_system;
pub mod conn;
pub mod error;
pub mod event;
pub mod host;
mod util;

#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use futures::{Future, IntoFuture};
    use tokio_core::reactor::Core;
    use tokio_postgres::Connection;

    use chat::{Chat, CreateChat};
    use chat_system::ChatSystem;
    use conn::database_connection;
    use event::{CreateEvent, Event};
    use error::EventError;

    #[test]
    fn can_establish_db_connection() {
        with_database(|connection| Ok(connection).into_future())
    }

    #[test]
    fn can_create_and_delete_chat_system() {
        with_database(|connection| with_chat_system(connection, 2, |tup| Ok(tup).into_future()))
    }

    #[test]
    fn can_create_and_delete_event() {
        with_database(|connection| {
            with_chat_system(connection, 3, |(chat_system, connection)| {
                with_event(chat_system, connection, Vec::new(), |tup| {
                    Ok(tup).into_future()
                })
            })
        })
    }

    #[test]
    fn can_create_and_delete_event_with_hosts() {
        with_database(|connection| {
            with_chat_system(connection, 4, |(chat_system, connection)| {
                with_event(chat_system, connection, vec![1, 2, 3], |tup| {
                    Ok(tup).into_future()
                })
            })
        })
    }

    #[test]
    fn can_create_and_delete_chat() {
        with_database(|connection| {
            with_chat_system(connection, 5, |(chat_system, connection)| {
                with_chat(chat_system, connection, 6, |tup| Ok(tup).into_future())
            })
        })
    }

    #[test]
    fn can_find_event_from_associated_chat() {
        with_database(|connection| {
            with_chat_system(connection, 7, |(chat_system, connection)| {
                with_chat(
                    chat_system,
                    connection,
                    8,
                    |(chat, chat_system, connection)| {
                        let chat_clone = chat.clone();
                        with_event(
                            chat_system,
                            connection,
                            vec![1, 2, 3],
                            move |(event, chat_system, connection)| {
                                chat_clone.get_events(connection).then(|res| match res {
                                    Ok((events, connection)) => {
                                        assert_eq!(events.len(), 1);
                                        assert!(
                                            events
                                                .get(0)
                                                .map(|event| assert_eq!(event.hosts().len(), 3))
                                                .is_some()
                                        );
                                        Ok((event, chat_system, connection))
                                    }
                                    Err((e, connection)) => {
                                        Err((e, event, chat_system, connection))
                                    }
                                })
                            },
                        ).then(move |res| match res {
                            Ok((chat_system, connection)) => Ok((chat, chat_system, connection)),
                            Err((error, chat_system, connection)) => {
                                Err((error, chat, chat_system, connection))
                            }
                        })
                    },
                )
            })
        })
    }

    fn with_database<F, G>(f: F)
    where
        F: FnOnce(Connection) -> G,
        G: Future<Item = Connection, Error = (EventError, Connection)>,
    {
        let mut core = Core::new().unwrap();

        let fut = database_connection(core.handle()).and_then(|conn| f(conn).map_err(|(e, _)| e));

        core.run(fut).unwrap();
    }

    fn with_chat_system<F, G>(
        connection: Connection,
        id: i64,
        f: F,
    ) -> Box<Future<Item = Connection, Error = (EventError, Connection)>>
    where
        F: FnOnce((ChatSystem, Connection)) -> G + 'static,
        G: Future<Item = (ChatSystem, Connection), Error = (EventError, ChatSystem, Connection)>
            + 'static,
    {
        Box::new(ChatSystem::create(id, connection).and_then(|tup| {
            f(tup)
                .or_else(|(error, chat_system, connection)| {
                    chat_system
                        .delete(connection)
                        .and_then(move |(count, connection)| {
                            assert_eq!(count, 1);
                            Err((error, connection))
                        })
                })
                .and_then(|(chat_system, connection)| chat_system.delete(connection))
                .map(|(count, connection)| {
                    assert_eq!(count, 1);
                    connection
                })
        }))
    }

    fn with_event<F, G>(
        chat_system: ChatSystem,
        connection: Connection,
        hosts: Vec<i64>,
        f: F,
    ) -> Box<Future<Item = (ChatSystem, Connection), Error = (EventError, ChatSystem, Connection)>>
    where
        F: FnOnce((Event, ChatSystem, Connection)) -> G + 'static,
        G: Future<
            Item = (Event, ChatSystem, Connection),
            Error = (EventError, Event, ChatSystem, Connection),
        >
            + 'static,
    {
        let new_event = CreateEvent {
            start_date: Utc::now(),
            end_date: Utc::now(),
            title: "Hey!".to_owned(),
            description: "Whoah hi".to_owned(),
            hosts: hosts,
        };

        println!("About to create new event");

        Box::new(
            new_event
                .create(&chat_system, connection)
                .then(|res| match res {
                    Ok((event, connection)) => Ok((event, chat_system, connection)),
                    Err((error, connection)) => Err((error, chat_system, connection)),
                })
                .and_then(|tup| {
                    f(tup)
                        .or_else(|(error, event, chat_system, connection)| {
                            event.delete(connection).then(move |res| match res {
                                Ok((count, connection)) => {
                                    assert_eq!(count, 1);
                                    Err((error, chat_system, connection))
                                }
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                        .and_then(|(event, chat_system, connection)| {
                            event.delete(connection).then(|res| match res {
                                Ok((count, connection)) => {
                                    assert_eq!(count, 1);
                                    Ok((chat_system, connection))
                                }
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                }),
        )
    }

    fn with_chat<F, G>(
        chat_system: ChatSystem,
        connection: Connection,
        chat_id: i64,
        f: F,
    ) -> Box<Future<Item = (ChatSystem, Connection), Error = (EventError, ChatSystem, Connection)>>
    where
        F: FnOnce((Chat, ChatSystem, Connection)) -> G + 'static,
        G: Future<
            Item = (Chat, ChatSystem, Connection),
            Error = (EventError, Chat, ChatSystem, Connection),
        >
            + 'static,
    {
        let new_chat = CreateChat { chat_id };

        println!("About to create new chat");

        Box::new(
            new_chat
                .create(&chat_system, connection)
                .then(|res| match res {
                    Ok((chat, connection)) => Ok((chat, chat_system, connection)),
                    Err((error, connection)) => Err((error, chat_system, connection)),
                })
                .and_then(|tup| {
                    f(tup)
                        .or_else(|(error, chat, chat_system, connection)| {
                            chat.delete(connection).then(move |res| match res {
                                Ok((count, connection)) => {
                                    assert_eq!(count, 1);
                                    Err((error, chat_system, connection))
                                }
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                        .and_then(|(chat, chat_system, connection)| {
                            chat.delete(connection).then(|res| match res {
                                Ok((count, connection)) => {
                                    assert_eq!(count, 1);
                                    Ok((chat_system, connection))
                                }
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                }),
        )
    }
}
