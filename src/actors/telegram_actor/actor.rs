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

//! This module defines the handlers for incoming messages to the `TelegramActor`.
//!
//! There are two classes of messages here. The first class is messages the actor sends itself. The
//! second is messages other actors send this actor. This actor sends itself messages in order to
//! handle incoming events like Telegram Updates, or a failed Telegram Update Stream. Other actors
//! send this actor messages as a proxy to talk to Telegram.

use actix::{Actor, Address, Arbiter, AsyncContext, Context, Handler, Supervised};
use futures::{Future, Stream};
use futures::stream::{iter_ok, repeat};
use telebot::functions::*;
use telebot::objects::Update;
use telebot::RcBot;

use error::{EventError, EventErrorKind};
use super::messages::*;
use super::TelegramActor;

impl Actor for TelegramActor {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        debug!("Started telegram message actor");
    }
}

impl Supervised for TelegramActor {
    fn restarting(&mut self, ctx: &mut <Self as Actor>::Context) {
        debug!("Restarting telegram message actor!");
        self.bot = RcBot::new(Arbiter::handle().clone(), &self.bot.inner.key);

        ctx.address::<Address<_>>().send(StartStreaming);
    }
}

impl Handler<NewEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        self.new_event(msg.0);
    }
}

impl Handler<UpdateEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: UpdateEvent, _: &mut Self::Context) -> Self::Result {
        self.update_event(msg.0);
    }
}

impl Handler<EventSoon> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventSoon, _: &mut Self::Context) -> Self::Result {
        self.event_soon(msg.0);
    }
}

impl Handler<EventStarted> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventStarted, _: &mut Self::Context) -> Self::Result {
        self.event_started(msg.0);
    }
}

impl Handler<EventOver> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventOver, _: &mut Self::Context) -> Self::Result {
        self.event_over(msg.0);
    }
}
impl Handler<Result<TgUpdate, EventError>> for TelegramActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: Result<TgUpdate, EventError>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        debug!("Handling update");
        match msg {
            Ok(tg_update) => self.handle_update(tg_update.update),
            Err(err) => {
                error!("Error {:?}", err);
                ctx.address::<Address<_>>().send(StartStreaming);
            }
        }
    }
}

impl Handler<StartStreaming> for TelegramActor {
    type Result = ();

    fn handle(&mut self, _: StartStreaming, ctx: &mut Self::Context) -> Self::Result {
        let addr: Address<_> = ctx.address();

        Arbiter::handle().spawn(
            bot_stream(self.bot.clone())
                .then(move |res| match res {
                    Ok((bot, update)) => addr.call_fut(Ok(TgUpdate { bot, update })),
                    Err(e) => addr.call_fut(Err(e)),
                })
                .map_err(|e| error!("Error: {:?}", e))
                .for_each(|_| Ok(())),
        )
    }
}

/// define a static stream for an `RcBot`, in order to use this as a future spawned in the actor's
/// context.
fn bot_stream(bot: RcBot) -> impl Stream<Item = (RcBot, Update), Error = EventError> {
    repeat::<RcBot, EventError>(bot)
        .and_then(move |bot| {
            debug!("Querying for updates");
            bot.get_updates()
                .offset(bot.inner.last_id.get())
                .timeout(bot.inner.timeout.get() as i64)
                .send()
                .map_err(|e| e.context(EventErrorKind::Telegram).into())
        })
        .map(|(bot, updates)| iter_ok(updates.0).map(move |update| (bot.clone(), update)))
        .flatten()
        .and_then(move |(bot, update)| {
            if bot.inner.last_id.get() < update.update_id as u32 + 1 {
                bot.inner.last_id.set(update.update_id as u32 + 1);
            }

            Ok((bot, update))
        })
        .map_err(|e| {
            error!("Error in bot stream: {:?}", e);
            e
        })
}
