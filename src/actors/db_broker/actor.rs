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

use actix::fut::wrap_future;
use actix::{Actor, Addr, Arbiter, AsyncContext, Context, Handler, ResponseActFuture, Unsync};
use futures::Future;
use tokio_postgres::Connection;

use super::messages::*;
use super::DbBroker;
use conn::connect_to_database;
use error::EventError;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use models::edit_event_link::EditEventLink;
use models::event::Event;
use models::new_event_link::NewEventLink;
use models::user::User;

type FutureResponse<I> = ResponseActFuture<DbBroker, I, EventError>;

impl DbBroker {
    /// Given a function that returns a future, create an ActorFuture that will run in the context
    /// of the Broker, providing a Connection to the future and taking it back afterwards
    fn wrap_fut<I, Fut, Func>(
        &self,
        f: Func,
        ctx: &mut <Self as Actor>::Context,
    ) -> FutureResponse<I>
    where
        Func: FnOnce(Connection) -> Fut + 'static,
        Fut: Future<Item = (I, Connection), Error = (EventError, Connection)> + 'static,
        I: 'static,
    {
        let addr: Addr<Unsync, _> = ctx.address();

        Box::new(wrap_future(
            self.connections
                .clone()
                .map_err(Err)
                .and_then(move |connection| f(connection).map_err(Ok))
                .then(move |full_res| match full_res {
                    Ok((item, connection)) => {
                        addr.do_send(Ready { connection });
                        Ok(item)
                    }
                    Err(res) => match res {
                        Ok((err, connection)) => {
                            addr.do_send(Ready { connection });
                            Err(err)
                        }
                        Err(err) => Err(err),
                    },
                }),
        ))
    }
}

impl Actor for DbBroker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db_broker: Addr<Unsync, _> = ctx.address();

        for _ in 0..self.num_connections {
            let fut = connect_to_database(self.db_url.clone(), Arbiter::handle().clone())
                .join(Ok(db_broker.clone()))
                .and_then(move |(connection, db_broker)| {
                    db_broker.do_send(Ready { connection });
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
    type Result = FutureResponse<ChatSystem>;

    fn handle(&mut self, msg: NewChannel, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::insert_channel(msg.channel_id, connection),
            ctx,
        )
    }
}

impl Handler<DeleteChannel> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: DeleteChannel, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::delete_chat_system(msg.channel_id, connection),
            ctx,
        )
    }
}

impl Handler<NewChat> for DbBroker {
    type Result = FutureResponse<Chat>;

    fn handle(&mut self, msg: NewChat, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::insert_chat(msg.channel_id, msg.chat_id, connection),
            ctx,
        )
    }
}

impl Handler<NewUser> for DbBroker {
    type Result = FutureResponse<User>;

    fn handle(&mut self, msg: NewUser, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
                DbBroker::new_user(msg.chat_id, msg.user_id, msg.username, connection)
            },
            ctx,
        )
    }
}

impl Handler<NewRelation> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: NewRelation, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
                DbBroker::new_user_chat_relation(msg.chat_id, msg.user_id, connection)
            },
            ctx,
        )
    }
}

impl Handler<NewEvent> for DbBroker {
    type Result = FutureResponse<Event>;

    fn handle(&mut self, msg: NewEvent, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
                DbBroker::insert_event(
                    msg.system_id,
                    msg.title,
                    msg.description,
                    msg.start_date,
                    msg.end_date,
                    msg.hosts,
                    connection,
                )
            },
            ctx,
        )
    }
}

impl Handler<EditEvent> for DbBroker {
    type Result = FutureResponse<Event>;

    fn handle(&mut self, msg: EditEvent, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
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
            },
            ctx,
        )
    }
}

impl Handler<LookupEventsByChatId> for DbBroker {
    type Result = FutureResponse<Vec<Event>>;

    fn handle(&mut self, msg: LookupEventsByChatId, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_events_by_chat_id(msg.chat_id, connection),
            ctx,
        )
    }
}

impl Handler<LookupEvent> for DbBroker {
    type Result = FutureResponse<Event>;

    fn handle(&mut self, msg: LookupEvent, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::lookup_event(msg.event_id, connection),
            ctx,
        )
    }
}

impl Handler<LookupEventsByUserId> for DbBroker {
    type Result = FutureResponse<Vec<Event>>;

    fn handle(&mut self, msg: LookupEventsByUserId, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::lookup_events_by_user_id(msg.user_id, connection),
            ctx,
        )
    }
}

impl Handler<DeleteEvent> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: DeleteEvent, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::delete_event(msg.event_id, connection),
            ctx,
        )
    }
}

impl Handler<GetEventsInRange> for DbBroker {
    type Result = FutureResponse<Vec<Event>>;

    fn handle(&mut self, msg: GetEventsInRange, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
                DbBroker::get_events_in_range(msg.start_date, msg.end_date, connection)
            },
            ctx,
        )
    }
}

impl Handler<LookupSystem> for DbBroker {
    type Result = FutureResponse<ChatSystem>;

    fn handle(&mut self, msg: LookupSystem, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_system_by_id(msg.system_id, connection),
            ctx,
        )
    }
}

impl Handler<LookupSystemByChannel> for DbBroker {
    type Result = FutureResponse<ChatSystem>;

    fn handle(&mut self, msg: LookupSystemByChannel, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_system_by_channel(msg.0, connection),
            ctx,
        )
    }
}

impl Handler<GetEventsForSystem> for DbBroker {
    type Result = FutureResponse<Vec<Event>>;

    fn handle(&mut self, msg: GetEventsForSystem, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_events_for_system(msg.system_id, connection),
            ctx,
        )
    }
}

impl Handler<GetUsersWithChats> for DbBroker {
    type Result = FutureResponse<Vec<(User, Chat)>>;

    fn handle(&mut self, _: GetUsersWithChats, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_users_with_chats(connection),
            ctx,
        )
    }
}

impl Handler<StoreEditEventLink> for DbBroker {
    type Result = FutureResponse<EditEventLink>;

    fn handle(&mut self, msg: StoreEditEventLink, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
                DbBroker::store_edit_event_link(
                    msg.user_id,
                    msg.system_id,
                    msg.event_id,
                    msg.secret,
                    connection,
                )
            },
            ctx,
        )
    }
}

impl Handler<LookupEditEventLink> for DbBroker {
    type Result = FutureResponse<EditEventLink>;

    fn handle(&mut self, msg: LookupEditEventLink, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_edit_event_link(msg.0, connection),
            ctx,
        )
    }
}

impl Handler<DeleteEditEventLink> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: DeleteEditEventLink, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::delete_edit_event_link(msg.id, connection),
            ctx,
        )
    }
}

impl Handler<StoreEventLink> for DbBroker {
    type Result = FutureResponse<NewEventLink>;

    fn handle(&mut self, msg: StoreEventLink, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| {
                DbBroker::store_event_link(msg.user_id, msg.system_id, msg.secret, connection)
            },
            ctx,
        )
    }
}

impl Handler<LookupEventLink> for DbBroker {
    type Result = FutureResponse<NewEventLink>;

    fn handle(&mut self, msg: LookupEventLink, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_event_link(msg.0, connection),
            ctx,
        )
    }
}

impl Handler<DeleteEventLink> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: DeleteEventLink, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::delete_event_link(msg.id, connection),
            ctx,
        )
    }
}

impl Handler<LookupUser> for DbBroker {
    type Result = FutureResponse<User>;

    fn handle(&mut self, msg: LookupUser, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::lookup_user(msg.0, connection),
            ctx,
        )
    }
}

impl Handler<GetSystemsWithChats> for DbBroker {
    type Result = FutureResponse<Vec<(ChatSystem, Chat)>>;

    fn handle(&mut self, _: GetSystemsWithChats, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::get_systems_with_chats(connection),
            ctx,
        )
    }
}

impl Handler<RemoveUserChat> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: RemoveUserChat, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::remove_user_chat(msg.0, msg.1, connection),
            ctx,
        )
    }
}

impl Handler<DeleteUserByUserId> for DbBroker {
    type Result = FutureResponse<()>;

    fn handle(&mut self, msg: DeleteUserByUserId, ctx: &mut Self::Context) -> Self::Result {
        self.wrap_fut(
            move |connection| DbBroker::delete_user_by_user_id(msg.0, connection),
            ctx,
        )
    }
}
