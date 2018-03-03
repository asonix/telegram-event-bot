use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use actix::Address;
use futures::{Async, Future, Poll};
use futures::task;

use actors::db_actor::DbActor;
use error::EventError;

mod actor;
pub mod messages;

pub struct Addresses(Rc<RefCell<VecDeque<Address<DbActor>>>>);

impl Clone for Addresses {
    fn clone(&self) -> Self {
        Addresses(Rc::clone(&self.0))
    }
}

impl Default for Addresses {
    fn default() -> Self {
        Addresses(Rc::new(RefCell::new(VecDeque::default())))
    }
}

impl Future for Addresses {
    type Item = Address<DbActor>;
    type Error = EventError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(item) = self.0.borrow_mut().pop_front() {
            Ok(Async::Ready(item))
        } else {
            // busy wait until we have a connection to use
            task::current().notify();
            Ok(Async::NotReady)
        }
    }
}

pub struct DbBroker {
    num_connections: usize,
    db_url: String,
    db_actors: Addresses,
}

impl DbBroker {
    pub fn new(db_url: String, num_connections: usize) -> Self {
        DbBroker {
            num_connections: num_connections,
            db_url: db_url,
            db_actors: Addresses::default(),
        }
    }
}
