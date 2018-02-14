use std::time::Duration;

use actix::{Actor, ActorContext, Address, Arbiter, AsyncContext, Context, Handler};
use futures::{Future, Stream};
use tokio_core::reactor::Interval;

use super::messages::*;
use super::Timer;

impl Actor for Timer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.add_stream(
            Interval::new(Duration::from_secs(60 * 60), &Arbiter::handle())
                .unwrap()
                .map(|_| NextHour)
                .map_err(|_| Shutdown),
        );
    }
}

impl Handler<Result<NextHour, Shutdown>> for Timer {
    type Result = ();

    fn handle(&mut self, res: Result<NextHour, Shutdown>, ctx: &mut Self::Context) -> Self::Result {
        match res {
            Ok(_) => {
                let address: Address<_> = ctx.address();

                let fut = self.get_next_hour()
                    .map(move |events| {
                        address.send(Events { events });
                    })
                    .map_err(|e| println!("Error: {:?}", e));

                Arbiter::handle().spawn(fut);
            }
            Err(_) => {
                ctx.stop();
            }
        }
    }
}

impl Handler<Events> for Timer {
    type Result = ();

    fn handle(&mut self, msg: Events, _: &mut Self::Context) -> Self::Result {
        let Events { events } = msg;

        self.set_deleters(&events);
        self.set_notifiers(events);
    }
}
