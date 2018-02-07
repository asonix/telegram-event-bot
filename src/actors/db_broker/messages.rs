use actix::{Address, ResponseType};

use actors::db_actor::DbActor;

pub struct Ready {
    pub db_actor: Address<DbActor>,
}

impl ResponseType for Ready {
    type Item = ();
    type Error = ();
}
