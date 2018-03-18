/*
 * This file is part of Event Web
 *
 * Event Web is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Event Web is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with Event Web.  If not, see <https://www.gnu.org/licenses/>.
 */

extern crate actix;
extern crate event_web;

use actix::{Actor, Context, Handler, System};
use event_web::{EditEvent, Event, FrontendError, FrontendErrorKind, LookupEvent, NewEvent};

#[derive(Copy, Clone, Debug)]
struct MyHandler;

impl Actor for MyHandler {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for MyHandler {
    type Result = Result<(), FrontendError>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        Ok(println!("Event: {:?}", msg.0))
    }
}

impl Handler<EditEvent> for MyHandler {
    type Result = Result<(), FrontendError>;

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        Ok(println!("Event: {:?}", msg.0))
    }
}

impl Handler<LookupEvent> for MyHandler {
    type Result = Result<Event, FrontendError>;

    fn handle(&mut self, _: LookupEvent, _: &mut Self::Context) -> Self::Result {
        Err(FrontendErrorKind::Canceled.into())
    }
}

fn main() {
    let sys = System::new("womp");

    event_web::start(MyHandler.start(), "0.0.0.0:8000", None);

    sys.run();
}
