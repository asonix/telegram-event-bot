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

use std::time::Duration;

use actix::{Actor, Address, Arbiter, AsyncContext, Context, Handler};
use futures::{Future, Stream};
use tokio_core::reactor::Interval;

use super::messages::*;
use super::Timer;

impl Actor for Timer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("Started Timer Actor");
        // Every 30 minutes, check for events happening in the next hour
        ctx.add_stream(
            Interval::new(Duration::from_secs(30 * 60), &Arbiter::handle())
                .unwrap()
                .map(|_| NextHour)
                .map_err(|_| Shutdown),
        );

        // Every 30 seconds, check if any events have any pending actions
        ctx.add_stream(
            Interval::new(Duration::from_secs(30), &Arbiter::handle())
                .unwrap()
                .map(|_| Migrate)
                .map_err(|_| MigrateError),
        );

        ctx.address::<Address<_>>().send(Ok(NextHour));
        ctx.address::<Address<_>>().send(Ok(Migrate));
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
                error!("Interval for NextHour errored");
                ctx.add_stream(
                    Interval::new(Duration::from_secs(60 * 60), &Arbiter::handle())
                        .unwrap()
                        .map(|_| NextHour)
                        .map_err(|_| Shutdown),
                );
            }
        }
    }
}

impl Handler<Result<Migrate, MigrateError>> for Timer {
    type Result = ();

    fn handle(
        &mut self,
        res: Result<Migrate, MigrateError>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        match res {
            Ok(_) => {
                self.migrate_events();
            }
            Err(_) => {
                error!("Interval for Migrate errored");
                ctx.add_stream(
                    Interval::new(Duration::from_secs(30), &Arbiter::handle())
                        .unwrap()
                        .map(|_| Migrate)
                        .map_err(|_| MigrateError),
                );
            }
        }
    }
}

impl Handler<Events> for Timer {
    type Result = ();

    fn handle(&mut self, msg: Events, _: &mut Self::Context) -> Self::Result {
        self.handle_events(msg.events);
    }
}

impl Handler<UpdateEvent> for Timer {
    type Result = ();

    fn handle(&mut self, msg: UpdateEvent, _: &mut Self::Context) -> Self::Result {
        self.update_event(msg.event);
    }
}
