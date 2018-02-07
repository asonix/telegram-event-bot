use actix::{Actor, Address, Arbiter, AsyncContext, Context, Handler, ResponseFuture, ResponseType};
use actix::fut::wrap_future;
use failure;
use failure::Fail;
use futures::{Future, IntoFuture};

use actors::db_actor::DbActor;
use conn::connect_to_database;
use error::EventErrorKind;
use super::DbBroker;
use super::messages::*;

impl Actor for DbBroker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db_broker: Address<_> = ctx.address();

        for _ in 0..self.num_connections {
            let fut = connect_to_database(self.db_url.clone(), Arbiter::handle().clone())
                .join(Ok(db_broker.clone()))
                .and_then(move |(connection, db_broker)| {
                    let db_actor: Address<_> = DbActor::new(db_broker.clone(), connection).start();

                    db_broker.send(Ready { db_actor });
                    Ok(())
                })
                .map_err(|_| ());

            Arbiter::handle().spawn(fut);
        }
    }
}

impl Handler<Ready> for DbBroker {
    type Result = ();

    fn handle(&mut self, msg: Ready, _: &mut Self::Context) -> Self::Result {
        self.db_actors.push_back(msg.db_actor);
    }
}

impl<T> Handler<T> for DbBroker
where
    DbActor: Handler<T>,
    T: ResponseType + 'static,
    <T as ResponseType>::Error: From<EventErrorKind>
        + From<failure::Context<EventErrorKind>>
        + 'static,
{
    type Result = ResponseFuture<Self, T>;

    fn handle(&mut self, msg: T, _: &mut Self::Context) -> Self::Result {
        if let Some(db_actor) = self.db_actors.pop_front() {
            Box::new(wrap_future(db_actor.call_fut(msg).then(
                |msg_res| match msg_res {
                    Ok(res) => res,
                    Err(err) => Err(err.context(EventErrorKind::Cancelled).into()),
                },
            )))
        } else {
            Box::new(wrap_future::<_, Self>(
                Err(EventErrorKind::NoAvailableConnection.into()).into_future(),
            ))
        }
    }
}
