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

//! This module defines all the Handler and Actor traits for the `DbBroker` type.

use actix::{Actor, ActorFuture, Address, Arbiter, AsyncContext, Context, Handler, ResponseFuture};
use actix::fut::wrap_future;
use futures::Future;
use tokio_postgres::Connection;

use conn::connect_to_database;
use error::EventError;
use super::DbBroker;
use super::messages::*;

impl DbBroker {
    /// Given a function that returns a future, create an ActorFuture that will run in the context
    /// of the Broker, providing a Connection to the future and taking it back afterwards
    fn wrap_fut<I, Fut, Func>(
        &self,
        f: Func,
    ) -> Box<ActorFuture<Item = I, Error = EventError, Actor = Self>>
    where
        Func: FnOnce(Connection) -> Fut + 'static,
        Fut: Future<Item = (I, Connection), Error = (EventError, Connection)> + 'static,
    {
        Box::new(
            wrap_future::<_, Self>(
                self.connections
                    .clone()
                    .map_err(Err)
                    .and_then(move |connection| f(connection).map_err(Ok)),
            ).map(|(item, connection), db_broker, _| {
                db_broker.connections.0.borrow_mut().push_front(connection);
                debug!(
                    "Restored db connection, total available connections: {}",
                    db_broker.connections.0.borrow().len()
                );
                item
            })
                .map_err(|res, db_broker, _| match res {
                    Ok((error, connection)) => {
                        db_broker.connections.0.borrow_mut().push_front(connection);
                        debug!(
                            "Restored db connection, total available connections: {}",
                            db_broker.connections.0.borrow().len()
                        );

                        error
                    }
                    Err(error) => error,
                }),
        )
    }
}

impl Actor for DbBroker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db_broker: Address<_> = ctx.address();

        for _ in 0..self.num_connections {
            let fut = connect_to_database(self.db_url.clone(), Arbiter::handle().clone())
                .join(Ok(db_broker.clone()))
                .and_then(move |(connection, db_broker)| {
                    db_broker.send(Ready { connection });
                    Ok(())
                })
                .map_err(|e| error!("Error: {:?}", e));

            Arbiter::handle().spawn(fut);
        }
    }
}

impl Handler<Ready> for DbBroker {
    type Result = ();

    fn handle(&mut self, msg: Ready, _: &mut Self::Context) -> Self::Result {
        self.connections.0.borrow_mut().push_back(msg.connection);
        debug!(
            "Restored db connection, total available connections: {}",
            self.connections.0.borrow().len()
        );
    }
}
impl Handler<NewChannel> for DbBroker {
    type Result = ResponseFuture<Self, NewChannel>;

    fn handle(&mut self, msg: NewChannel, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::insert_channel(msg.channel_id, connection))
    }
}

impl Handler<DeleteChannel> for DbBroker {
    type Result = ResponseFuture<Self, DeleteChannel>;

    fn handle(&mut self, msg: DeleteChannel, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::delete_chat_system(msg.channel_id, connection))
    }
}

impl Handler<NewChat> for DbBroker {
    type Result = ResponseFuture<Self, NewChat>;

    fn handle(&mut self, msg: NewChat, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::insert_chat(msg.channel_id, msg.chat_id, connection)
        })
    }
}

impl Handler<NewUser> for DbBroker {
    type Result = ResponseFuture<Self, NewUser>;

    fn handle(&mut self, msg: NewUser, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::new_user(msg.chat_id, msg.user_id, msg.username, connection)
        })
    }
}

impl Handler<NewRelation> for DbBroker {
    type Result = ResponseFuture<Self, NewRelation>;

    fn handle(&mut self, msg: NewRelation, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::new_user_chat_relation(msg.chat_id, msg.user_id, connection)
        })
    }
}

impl Handler<NewEvent> for DbBroker {
    type Result = ResponseFuture<Self, NewEvent>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::insert_event(
                msg.system_id,
                msg.title,
                msg.description,
                msg.start_date,
                msg.end_date,
                msg.hosts,
                connection,
            )
        })
    }
}

impl Handler<EditEvent> for DbBroker {
    type Result = ResponseFuture<Self, EditEvent>;

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::edit_event(
                msg.id,
                msg.system_id,
                msg.title,
                msg.description,
                msg.start_date,
                msg.end_date,
                msg.hosts,
                connection,
            )
        })
    }
}

impl Handler<LookupEventsByChatId> for DbBroker {
    type Result = ResponseFuture<Self, LookupEventsByChatId>;

    fn handle(&mut self, msg: LookupEventsByChatId, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_events_by_chat_id(msg.chat_id, connection))
    }
}

impl Handler<LookupEvent> for DbBroker {
    type Result = ResponseFuture<Self, LookupEvent>;

    fn handle(&mut self, msg: LookupEvent, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::lookup_event(msg.event_id, connection))
    }
}

impl Handler<LookupEventsByUserId> for DbBroker {
    type Result = ResponseFuture<Self, LookupEventsByUserId>;

    fn handle(&mut self, msg: LookupEventsByUserId, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::lookup_events_by_user_id(msg.user_id, connection))
    }
}

impl Handler<DeleteEvent> for DbBroker {
    type Result = ResponseFuture<Self, DeleteEvent>;

    fn handle(&mut self, msg: DeleteEvent, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::delete_event(msg.event_id, connection))
    }
}

impl Handler<GetEventsInRange> for DbBroker {
    type Result = ResponseFuture<Self, GetEventsInRange>;

    fn handle(&mut self, msg: GetEventsInRange, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::get_events_in_range(msg.start_date, msg.end_date, connection)
        })
    }
}

impl Handler<LookupSystem> for DbBroker {
    type Result = ResponseFuture<Self, LookupSystem>;

    fn handle(&mut self, msg: LookupSystem, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_system_by_id(msg.system_id, connection))
    }
}

impl Handler<LookupSystemByChannel> for DbBroker {
    type Result = ResponseFuture<Self, LookupSystemByChannel>;

    fn handle(&mut self, msg: LookupSystemByChannel, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_system_by_channel(msg.0, connection))
    }
}

impl Handler<GetEventsForSystem> for DbBroker {
    type Result = ResponseFuture<Self, GetEventsForSystem>;

    fn handle(&mut self, msg: GetEventsForSystem, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_events_for_system(msg.system_id, connection))
    }
}

impl Handler<GetUsersWithChats> for DbBroker {
    type Result = ResponseFuture<Self, GetUsersWithChats>;

    fn handle(&mut self, _: GetUsersWithChats, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_users_with_chats(connection))
    }
}

impl Handler<StoreEditEventLink> for DbBroker {
    type Result = ResponseFuture<Self, StoreEditEventLink>;

    fn handle(&mut self, msg: StoreEditEventLink, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::store_edit_event_link(
                msg.user_id,
                msg.system_id,
                msg.event_id,
                msg.secret,
                connection,
            )
        })
    }
}

impl Handler<LookupEditEventLink> for DbBroker {
    type Result = ResponseFuture<Self, LookupEditEventLink>;

    fn handle(&mut self, msg: LookupEditEventLink, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_edit_event_link(msg.0, connection))
    }
}

impl Handler<DeleteEditEventLink> for DbBroker {
    type Result = ResponseFuture<Self, DeleteEditEventLink>;

    fn handle(&mut self, msg: DeleteEditEventLink, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::delete_edit_event_link(msg.id, connection))
    }
}

impl Handler<StoreEventLink> for DbBroker {
    type Result = ResponseFuture<Self, StoreEventLink>;

    fn handle(&mut self, msg: StoreEventLink, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| {
            DbBroker::store_event_link(msg.user_id, msg.system_id, msg.secret, connection)
        })
    }
}

impl Handler<LookupEventLink> for DbBroker {
    type Result = ResponseFuture<Self, LookupEventLink>;

    fn handle(&mut self, msg: LookupEventLink, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_event_link(msg.0, connection))
    }
}

impl Handler<DeleteEventLink> for DbBroker {
    type Result = ResponseFuture<Self, DeleteEventLink>;

    fn handle(&mut self, msg: DeleteEventLink, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::delete_event_link(msg.id, connection))
    }
}

impl Handler<LookupUser> for DbBroker {
    type Result = ResponseFuture<Self, LookupUser>;

    fn handle(&mut self, msg: LookupUser, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::lookup_user(msg.0, connection))
    }
}

impl Handler<GetSystemsWithChats> for DbBroker {
    type Result = ResponseFuture<Self, GetSystemsWithChats>;

    fn handle(&mut self, _: GetSystemsWithChats, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::get_systems_with_chats(connection))
    }
}

impl Handler<RemoveUserChat> for DbBroker {
    type Result = ResponseFuture<Self, RemoveUserChat>;

    fn handle(&mut self, msg: RemoveUserChat, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::remove_user_chat(msg.0, msg.1, connection))
    }
}

impl Handler<DeleteUserByUserId> for DbBroker {
    type Result = ResponseFuture<Self, DeleteUserByUserId>;

    fn handle(&mut self, msg: DeleteUserByUserId, _: &mut Self::Context) -> Self::Result {
        self.wrap_fut(move |connection| DbBroker::delete_user_by_user_id(msg.0, connection))
    }
}
