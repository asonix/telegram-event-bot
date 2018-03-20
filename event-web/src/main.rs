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
extern crate futures;

use actix::{Actor, Context, Handler, Message, System};
use event_web::{EditEvent, FrontendErrorKind, LookupEvent, NewEvent, SendFutResponse};
use futures::IntoFuture;

#[derive(Copy, Clone, Debug)]
struct MyHandler;

impl Actor for MyHandler {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for MyHandler {
    type Result = SendFutResponse<NewEvent>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        println!("Event: {:?}", msg.0);

        SendFutResponse::new(Box::new(Ok(()).into_future()) as <NewEvent as Message>::Result)
    }
}

impl Handler<EditEvent> for MyHandler {
    type Result = SendFutResponse<EditEvent>;

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        println!("Event: {:?}", msg.0);

        SendFutResponse::new(Box::new(Ok(()).into_future()) as <EditEvent as Message>::Result)
    }
}

impl Handler<LookupEvent> for MyHandler {
    type Result = SendFutResponse<LookupEvent>;

    fn handle(&mut self, _: LookupEvent, _: &mut Self::Context) -> Self::Result {
        SendFutResponse::new(
            Box::new(Err(FrontendErrorKind::Canceled.into()).into_future())
                as <LookupEvent as Message>::Result,
        )
    }
}

fn main() {
    let sys = System::new("womp");

    event_web::start(MyHandler.start(), "0.0.0.0:8000", None);

    sys.run();
}
