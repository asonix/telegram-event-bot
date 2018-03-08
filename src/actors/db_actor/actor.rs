use actix::{Actor, ActorFuture, AsyncContext, Context, Handler, ResponseFuture};
use futures;
use tokio_postgres::Connection;

use actors::db_broker::messages::Ready;
use error::EventError;
use super::DbActor;
use super::messages::*;

impl Actor for DbActor {
    type Context = Context<Self>;
}

impl Handler<NewChannel> for DbActor {
    type Result = ResponseFuture<Self, NewChannel>;

    fn handle(&mut self, msg: NewChannel, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_channel(msg.channel_id))
    }
}

impl Handler<DeleteChannel> for DbActor {
    type Result = ResponseFuture<Self, DeleteChannel>;

    fn handle(&mut self, msg: DeleteChannel, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_chat_system(msg.channel_id))
    }
}

impl Handler<NewChat> for DbActor {
    type Result = ResponseFuture<Self, NewChat>;

    fn handle(&mut self, msg: NewChat, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_chat(msg.channel_id, msg.chat_id))
    }
}

impl Handler<NewUser> for DbActor {
    type Result = ResponseFuture<Self, NewUser>;

    fn handle(&mut self, msg: NewUser, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.new_user(msg.chat_id, msg.user_id, msg.username))
    }
}

impl Handler<NewRelation> for DbActor {
    type Result = ResponseFuture<Self, NewRelation>;

    fn handle(&mut self, msg: NewRelation, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.new_user_chat_relation(msg.chat_id, msg.user_id))
    }
}

impl Handler<NewEvent> for DbActor {
    type Result = ResponseFuture<Self, NewEvent>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_event(
            msg.system_id,
            msg.title,
            msg.description,
            msg.start_date,
            msg.end_date,
            msg.hosts,
        ))
    }
}

impl Handler<EditEvent> for DbActor {
    type Result = ResponseFuture<Self, EditEvent>;

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.edit_event(
            msg.id,
            msg.system_id,
            msg.title,
            msg.description,
            msg.start_date,
            msg.end_date,
            msg.hosts,
        ))
    }
}

impl Handler<LookupEventsByChatId> for DbActor {
    type Result = ResponseFuture<Self, LookupEventsByChatId>;

    fn handle(&mut self, msg: LookupEventsByChatId, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_events_by_chat_id(msg.chat_id))
    }
}

impl Handler<LookupEvent> for DbActor {
    type Result = ResponseFuture<Self, LookupEvent>;

    fn handle(&mut self, msg: LookupEvent, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.lookup_event(msg.event_id))
    }
}

impl Handler<LookupEventsByUserId> for DbActor {
    type Result = ResponseFuture<Self, LookupEventsByUserId>;

    fn handle(&mut self, msg: LookupEventsByUserId, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.lookup_events_by_user_id(msg.user_id))
    }
}

impl Handler<DeleteEvent> for DbActor {
    type Result = ResponseFuture<Self, DeleteEvent>;

    fn handle(&mut self, msg: DeleteEvent, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_event(msg.event_id))
    }
}

impl Handler<GetEventsInRange> for DbActor {
    type Result = ResponseFuture<Self, GetEventsInRange>;

    fn handle(&mut self, msg: GetEventsInRange, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_events_in_range(msg.start_date, msg.end_date))
    }
}

impl Handler<GetChatSystemByEventId> for DbActor {
    type Result = ResponseFuture<Self, GetChatSystemByEventId>;

    fn handle(&mut self, msg: GetChatSystemByEventId, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_chat_system_by_event_id(msg.event_id))
    }
}

impl Handler<LookupSystem> for DbActor {
    type Result = ResponseFuture<Self, LookupSystem>;

    fn handle(&mut self, msg: LookupSystem, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_system_by_id(msg.system_id))
    }
}

impl Handler<LookupSystemByChannel> for DbActor {
    type Result = ResponseFuture<Self, LookupSystemByChannel>;

    fn handle(&mut self, msg: LookupSystemByChannel, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_system_by_channel(msg.0))
    }
}

impl Handler<GetEventsForSystem> for DbActor {
    type Result = ResponseFuture<Self, GetEventsForSystem>;

    fn handle(&mut self, msg: GetEventsForSystem, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_events_for_system(msg.system_id))
    }
}

impl Handler<GetUsersWithChats> for DbActor {
    type Result = ResponseFuture<Self, GetUsersWithChats>;

    fn handle(&mut self, _: GetUsersWithChats, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_users_with_chats())
    }
}

impl Handler<StoreEditEventLink> for DbActor {
    type Result = ResponseFuture<Self, StoreEditEventLink>;

    fn handle(&mut self, msg: StoreEditEventLink, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.store_edit_event_link(
            msg.user_id,
            msg.system_id,
            msg.event_id,
            msg.secret,
        ))
    }
}

impl Handler<LookupEditEventLink> for DbActor {
    type Result = ResponseFuture<Self, LookupEditEventLink>;

    fn handle(&mut self, msg: LookupEditEventLink, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_edit_event_link(msg.0))
    }
}

impl Handler<DeleteEditEventLink> for DbActor {
    type Result = ResponseFuture<Self, DeleteEditEventLink>;

    fn handle(&mut self, msg: DeleteEditEventLink, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_edit_event_link(msg.id))
    }
}

impl Handler<StoreEventLink> for DbActor {
    type Result = ResponseFuture<Self, StoreEventLink>;

    fn handle(&mut self, msg: StoreEventLink, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.store_event_link(msg.user_id, msg.system_id, msg.secret))
    }
}

impl Handler<LookupEventLink> for DbActor {
    type Result = ResponseFuture<Self, LookupEventLink>;

    fn handle(&mut self, msg: LookupEventLink, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_event_link(msg.0))
    }
}

impl Handler<DeleteEventLink> for DbActor {
    type Result = ResponseFuture<Self, DeleteEventLink>;

    fn handle(&mut self, msg: DeleteEventLink, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_event_link(msg.id))
    }
}

impl Handler<LookupUser> for DbActor {
    type Result = ResponseFuture<Self, LookupUser>;

    fn handle(&mut self, msg: LookupUser, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.lookup_user(msg.0))
    }
}

impl Handler<GetSystemsWithChats> for DbActor {
    type Result = ResponseFuture<Self, GetSystemsWithChats>;

    fn handle(&mut self, _: GetSystemsWithChats, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.get_systems_with_chats())
    }
}

impl Handler<RemoveUserChat> for DbActor {
    type Result = ResponseFuture<Self, RemoveUserChat>;

    fn handle(&mut self, msg: RemoveUserChat, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.remove_user_chat(msg.0, msg.1))
    }
}

impl Handler<DeleteUserByUserId> for DbActor {
    type Result = ResponseFuture<Self, DeleteUserByUserId>;

    fn handle(&mut self, msg: DeleteUserByUserId, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_user_by_user_id(msg.0))
    }
}

impl DbActor {
    fn wrap_fut<I, F>(fut: F) -> Box<ActorFuture<Item = I, Error = EventError, Actor = Self>>
    where
        F: futures::Future<
            Item = (I, Connection),
            Error = Result<(EventError, Connection), EventError>,
        >
            + 'static,
    {
        use actix::fut::wrap_future;

        Box::new(
            wrap_future::<_, Self>(fut)
                .map(|(item, connection), db_actor, ctx| {
                    db_actor.connection = Some(connection);

                    db_actor.broker.send(Ready {
                        db_actor: ctx.address(),
                    });

                    item
                })
                .map_err(|res, db_actor, ctx| match res {
                    Ok((error, connection)) => {
                        db_actor.connection = Some(connection);

                        db_actor.broker.send(Ready {
                            db_actor: ctx.address(),
                        });

                        error
                    }
                    Err(error) => error,
                }),
        )
    }
}
