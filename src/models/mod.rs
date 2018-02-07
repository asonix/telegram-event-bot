pub mod chat;
pub mod chat_system;
pub mod event;
pub mod user;
mod util;

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};
    use chrono::offset::Utc;
    use futures::{Future, IntoFuture};
    use tokio_core::reactor::Core;
    use tokio_postgres::Connection;

    use error::{EventError, EventErrorKind};
    use super::chat::{Chat, CreateChat};
    use super::chat_system::ChatSystem;
    use super::conn::database_connection;
    use super::event::{CreateEvent, Event};
    use super::user::{CreateUser, User};

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
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_event(chat_system, connection, Vec::new(), |tup| {
                    Ok(tup).into_future()
                })
            })
        })
    }

    #[test]
    fn can_create_and_delete_event_with_hosts() {
        with_database(|connection| {
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_chat(
                    chat_system,
                    connection,
                    gen_id(),
                    |(chat, chat_system, connection)| {
                        let system_clone = chat_system.clone();

                        with_user(
                            chat,
                            connection,
                            gen_id(),
                            move |(user, chat, connection)| {
                                with_event(system_clone, connection, vec![user.clone()], |tup| {
                                    Ok(tup).into_future()
                                }).then(move |res| match res {
                                    Ok((_, connection)) => Ok((user, chat, connection)),
                                    Err((error, _, connection)) => {
                                        Err((error, user, chat, connection))
                                    }
                                })
                            },
                        ).then(move |res| match res {
                            Ok((chat, connection)) => Ok((chat, chat_system, connection)),
                            Err((error, chat, connection)) => {
                                Err((error, chat, chat_system, connection))
                            }
                        })
                    },
                )
            })
        })
    }

    #[test]
    fn can_create_and_delete_chat() {
        with_database(|connection| {
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_chat(chat_system, connection, gen_id(), |tup| {
                    Ok(tup).into_future()
                })
            })
        })
    }

    #[test]
    fn can_find_event_from_associated_chat() {
        with_database(|connection| {
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_chat(
                    chat_system,
                    connection,
                    gen_id(),
                    |(chat, chat_system, connection)| {
                        let chat_clone = chat.clone();
                        with_event(
                            chat_system,
                            connection,
                            vec![],
                            move |(event, chat_system, connection)| {
                                chat_clone.get_events(connection).then(|res| match res {
                                    Ok((_, connection)) => Ok((event, chat_system, connection)),
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

    #[test]
    fn can_lookup_entire_system_by_chat_id() {
        with_database(|connection| {
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_chat(
                    chat_system,
                    connection,
                    gen_id(),
                    |(chat, chat_system, connection)| {
                        let chat_clone = chat.clone();
                        with_event(
                            chat_system,
                            connection,
                            vec![],
                            move |(event, chat_system, connection)| {
                                chat_clone.get_system_with_events(connection).then(
                                    |res| match res {
                                        Ok((_, connection)) => Ok((event, chat_system, connection)),
                                        Err((e, connection)) => {
                                            Err((e, event, chat_system, connection))
                                        }
                                    },
                                )
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

    #[test]
    fn can_create_and_delete_user_given_chat() {
        with_database(|connection| {
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_chat(
                    chat_system,
                    connection,
                    gen_id(),
                    |(chat, chat_system, connection)| {
                        with_user(chat, connection, gen_id(), |tup| Ok(tup).into_future()).then(
                            move |res| match res {
                                Ok((chat, connection)) => Ok((chat, chat_system, connection)),
                                Err((error, chat, connection)) => {
                                    Err((error, chat, chat_system, connection))
                                }
                            },
                        )
                    },
                )
            })
        })
    }

    #[test]
    fn can_lookup_systems_by_user() {
        with_database(|connection| {
            with_chat_system(connection, gen_id(), |(chat_system, connection)| {
                with_chat(
                    chat_system,
                    connection,
                    gen_id(),
                    |(chat, chat_system, connection)| {
                        with_user(chat, connection, gen_id(), |(user, chat, connection)| {
                            user.get_systems(connection).then(move |res| match res {
                                Ok((systems, connection)) => {
                                    if systems.len() == 1 {
                                        Ok((user, chat, connection))
                                    } else {
                                        Err((EventErrorKind::Lookup.into(), user, chat, connection))
                                    }
                                }
                                Err((error, connection)) => Err((error, user, chat, connection)),
                            })
                        }).then(move |res| match res {
                            Ok((chat, connection)) => Ok((chat, chat_system, connection)),
                            Err((error, chat, connection)) => {
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
        channel_id: i64,
        f: F,
    ) -> Box<Future<Item = Connection, Error = (EventError, Connection)>>
    where
        F: FnOnce((ChatSystem, Connection)) -> G + 'static,
        G: Future<Item = (ChatSystem, Connection), Error = (EventError, ChatSystem, Connection)>
            + 'static,
    {
        Box::new(ChatSystem::create(channel_id, connection).and_then(|tup| {
            f(tup)
                .or_else(|(error, chat_system, connection)| {
                    chat_system
                        .delete(connection)
                        .and_then(move |(_, connection)| Err((error, connection)))
                })
                .and_then(|(chat_system, connection)| chat_system.delete(connection))
                .map(|(_, connection)| connection)
        }))
    }

    fn with_event<F, G>(
        chat_system: ChatSystem,
        connection: Connection,
        hosts: Vec<User>,
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
                                Ok((_, connection)) => Err((error, chat_system, connection)),
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                        .and_then(|(event, chat_system, connection)| {
                            event.delete(connection).then(|res| match res {
                                Ok((_, connection)) => Ok((chat_system, connection)),
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
                                Ok((_, connection)) => Err((error, chat_system, connection)),
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                        .and_then(|(chat, chat_system, connection)| {
                            chat.delete(connection).then(|res| match res {
                                Ok((_, connection)) => Ok((chat_system, connection)),
                                Err((e, connection)) => Err((e, chat_system, connection)),
                            })
                        })
                }),
        )
    }

    fn with_user<F, G>(
        chat: Chat,
        connection: Connection,
        user_id: i64,
        f: F,
    ) -> Box<Future<Item = (Chat, Connection), Error = (EventError, Chat, Connection)>>
    where
        F: FnOnce((User, Chat, Connection)) -> G + 'static,
        G: Future<Item = (User, Chat, Connection), Error = (EventError, User, Chat, Connection)>
            + 'static,
    {
        let new_user = CreateUser { user_id };

        Box::new(
            new_user
                .create(&chat, connection)
                .then(|res| match res {
                    Ok((user, connection)) => Ok((user, chat, connection)),
                    Err((error, connection)) => Err((error, chat, connection)),
                })
                .and_then(|tup| {
                    f(tup)
                        .or_else(|(error, user, chat, connection)| {
                            user.delete(connection).then(move |res| match res {
                                Ok((_, connection)) => Err((error, chat, connection)),
                                Err((e, connection)) => Err((e, chat, connection)),
                            })
                        })
                        .and_then(|(user, chat, connection)| {
                            user.delete(connection).then(|res| match res {
                                Ok((_, connection)) => Ok((chat, connection)),
                                Err((e, connection)) => Err((e, chat, connection)),
                            })
                        })
                }),
        )
    }

    fn gen_id() -> i64 {
        let mut rng = thread_rng();

        rng.gen::<i64>()
    }
}
