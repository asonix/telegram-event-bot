use actix::ResponseType;

use models::event::Event;

pub struct NextHour;

impl ResponseType for NextHour {
    type Item = ();
    type Error = ();
}

pub struct Events {
    pub events: Vec<Event>,
}

impl ResponseType for Events {
    type Item = ();
    type Error = ();
}

pub struct Shutdown;

impl ResponseType for Shutdown {
    type Item = ();
    type Error = ();
}

pub struct Migrate;

impl ResponseType for Migrate {
    type Item = ();
    type Error = ();
}

pub struct MigrateError;

impl ResponseType for MigrateError {
    type Item = ();
    type Error = ();
}
