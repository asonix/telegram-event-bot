use std::collections::VecDeque;

use actix::Address;

use actors::db_actor::DbActor;

mod actor;
pub mod messages;

pub struct DbBroker {
    num_connections: usize,
    db_url: String,
    db_actors: VecDeque<Address<DbActor>>,
}

impl DbBroker {
    pub fn new(db_url: String, num_connections: usize) -> Self {
        DbBroker {
            num_connections: num_connections,
            db_url: db_url,
            db_actors: VecDeque::new(),
        }
    }
}
