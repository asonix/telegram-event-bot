extern crate actix;
extern crate event_web;

use actix::{Actor, Context, Handler, System};
use event_web::NewEvent;

#[derive(Copy, Clone, Debug)]
struct MyHandler;

impl Actor for MyHandler {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for MyHandler {
    type Result = ();

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        println!("Event: {:?}", msg.0);
    }
}

fn main() {
    let sys = System::new("womp");

    event_web::start(MyHandler.start(), "127.0.0.1:8000", None);

    sys.run();
}
