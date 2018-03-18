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

use actix::{Actor, Context, Handler, ResponseFuture};
use actix::fut::wrap_future;
use event_web::{EditEvent, LookupEvent, NewEvent};

use super::EventActor;

impl Actor for EventActor {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for EventActor {
    type Result = ResponseFuture<Self, NewEvent>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future(self.new_event(msg.0, msg.1)))
    }
}

impl Handler<LookupEvent> for EventActor {
    type Result = ResponseFuture<Self, LookupEvent>;

    fn handle(&mut self, msg: LookupEvent, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future::<_, Self>(self.lookup_event(msg.0)))
    }
}

impl Handler<EditEvent> for EventActor {
    type Result = ResponseFuture<Self, EditEvent>;

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future(self.edit_event(msg.0, msg.1)))
    }
}
