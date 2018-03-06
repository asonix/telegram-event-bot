use actix::{Actor, Address, Arbiter, AsyncContext, Context, Handler, ResponseFuture, ResponseType};
use actix::fut::wrap_future;
use failure;
use futures::Future;

use actors::db_actor::DbActor;
use conn::connect_to_database;
use error::EventError;
use error::EventErrorKind;
use super::DbBroker;
use super::messages::*;
use util::flatten;

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
                .map_err(|e| error!("Error: {:?}", e));

            Arbiter::handle().spawn(fut);
        }
    }
}

impl Handler<Ready> for DbBroker {
    type Result = ();

    fn handle(&mut self, msg: Ready, _: &mut Self::Context) -> Self::Result {
        self.db_actors.0.borrow_mut().push_back(msg.db_actor);
        debug!(
            "Restored db connection, total available connections: {}",
            self.db_actors.0.borrow().len()
        );
    }
}

impl<T> Handler<T> for DbBroker
where
    DbActor: Handler<T>,
    T: ResponseType + 'static,
    <T as ResponseType>::Error: From<EventError>
        + From<failure::Context<EventErrorKind>>
        + From<EventErrorKind>
        + 'static,
{
    type Result = ResponseFuture<Self, T>;

    fn handle(&mut self, msg: T, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future(
            self.db_actors
                .clone()
                .map_err(T::Error::from)
                .and_then(|db_actor| db_actor.call_fut(msg).then(flatten::<T>)),
        ))
    }
}
