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
        with_chat_system(2, |tup| Ok(tup).into_future())
    }

    #[test]
    fn can_create_and_delete_event() {
        with_event(3, |tup| Ok(tup).into_future())
    }

    fn with_database<F, G>(f: F)
    where
        F: FnOnce(Connection) -> G,
        G: Future<Item = Connection, Error = EventError>,
    {
        let mut core = Core::new().unwrap();

        let fut = database_connection(core.handle()).and_then(f);

        core.run(fut).unwrap();
    }

    fn with_chat_system<F, G>(id: i64, f: F)
    where
        F: FnOnce((ChatSystem, Connection)) -> G,
        G: Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)>,
    {
        with_database(|connection| {
            ChatSystem::create(id, connection)
                .and_then(f)
                .and_then(|(chat_system, connection)| chat_system.delete(connection))
                .map(|(count, connection)| {
                    assert_eq!(count, 1);
                    connection
                })
                .map_err(|(e, _)| e)
        })
    }

    fn with_event<F, G>(id: i64, f: F)
    where
        F: FnOnce((Event, Connection)) -> G,
        G: Future<Item = (Event, Connection), Error = (EventError, Connection)>,
    {
        with_chat_system(id, |(chat_system, connection)| {
            let new_event = CreateEvent {
                start_date: Utc::now(),
                end_date: Utc::now(),
                title: "Hey!".to_owned(),
                description: "Whoah hi".to_owned(),
                hosts: Vec::new(),
            };

            new_event
                .create(&chat_system, connection)
                .map(|(event, connection)| {
                    println!("Event: {:?}", event);
                    (event, connection)
                })
                .and_then(f)
                .and_then(|(event, connection)| event.delete(connection))
                .map(|(count, connection)| {
                    assert_eq!(count, 1);
                    connection
                })
                .then(|res| match res {
                    Ok(connection) => Ok((chat_system, connection)),
                    Err((e, connection)) => Err((e, chat_system, connection)),
                })
                .or_else(|(e, chat_system, connection)| {
                    chat_system
                        .delete(connection)
                        .and_then(move |(_, connection)| Err((e, connection)))
                })
        })
    }
}
