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

use actix::{Actor, AsyncContext, Context, Handler, Message};
use actix::fut::wrap_future;
use event_web::{EditEvent, FrontendError, FrontendErrorKind, LookupEvent, NewEvent,
                SendFutResponse};
use failure::Fail;
use futures::Future;
use futures::sync::oneshot;

use super::EventActor;

fn flatten<T>(
    res: Result<Result<T, FrontendError>, oneshot::Canceled>,
) -> Result<T, FrontendError> {
    match res {
        Ok(res) => res,
        Err(e) => Err(e.context(FrontendErrorKind::Canceled).into()),
    }
}

fn split<F, T>(
    f: F,
    ctx: &mut <EventActor as Actor>::Context,
) -> oneshot::Receiver<Result<T, FrontendError>>
where
    F: Future<Item = T, Error = FrontendError> + 'static,
    T: 'static,
{
    let (tx, rx) = oneshot::channel();

    ctx.spawn(wrap_future(
        f.then(move |res| tx.send(res)).map(|_| ()).map_err(|_| ()),
    ));

    rx
}

impl Actor for EventActor {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for EventActor {
    type Result = SendFutResponse<NewEvent>;

    fn handle(&mut self, msg: NewEvent, ctx: &mut Self::Context) -> Self::Result {
        SendFutResponse::new(
            Box::new(split(self.new_event(msg.0, msg.1), ctx).then(flatten))
                as <NewEvent as Message>::Result,
        )
    }
}

impl Handler<LookupEvent> for EventActor {
    type Result = SendFutResponse<LookupEvent>;

    fn handle(&mut self, msg: LookupEvent, ctx: &mut Self::Context) -> Self::Result {
        SendFutResponse::new(Box::new(split(self.lookup_event(msg.0), ctx).then(flatten))
            as <LookupEvent as Message>::Result)
    }
}

impl Handler<EditEvent> for EventActor {
    type Result = SendFutResponse<EditEvent>;

    fn handle(&mut self, msg: EditEvent, ctx: &mut Self::Context) -> Self::Result {
        SendFutResponse::new(
            Box::new(split(self.edit_event(msg.0, msg.1), ctx).then(flatten))
                as <EditEvent as Message>::Result,
        )
    }
}
