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
        DbActor::wrap_fut(self.new_user(msg.chat_id, msg.user_id))
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
            msg.channel_id,
            msg.title,
            msg.description,
            msg.start_date,
            msg.end_date,
            msg.hosts,
        ))
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
