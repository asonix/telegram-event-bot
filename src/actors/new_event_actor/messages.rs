use actix::ResponseType;
use chrono::DateTime;
use chrono::offset::Utc;
use telebot::objects::Integer;

use error::EventError;
use models::event::Event;
use models::user::User;

pub struct NewTitle {
    pub user_id: Integer,
    pub channel_id: Integer,
    pub title: String,
    pub chat_id: Integer,
}

impl ResponseType for NewTitle {
    type Item = ();
    type Error = ();
}

pub struct AddDescription {
    pub user_id: Integer,
    pub description: String,
    pub chat_id: Integer,
}

impl ResponseType for AddDescription {
    type Item = ();
    type Error = EventError;
}

pub struct AddDate {
    pub user_id: Integer,
    pub start_date: DateTime<Utc>,
    pub chat_id: Integer,
}

impl ResponseType for AddDate {
    type Item = ();
    type Error = EventError;
}

pub struct AddEnd {
    pub user_id: Integer,
    pub end_date: DateTime<Utc>,
    pub chat_id: Integer,
}

impl ResponseType for AddEnd {
    type Item = ();
    type Error = EventError;
}

pub struct AddHost {
    pub user_id: Integer,
    pub host: User,
    pub chat_id: Integer,
}

impl ResponseType for AddHost {
    type Item = ();
    type Error = EventError;
}

pub struct Finalize {
    pub user_id: Integer,
    pub chat_id: Integer,
}

impl ResponseType for Finalize {
    type Item = Event;
    type Error = EventError;
}

pub struct Incoming {
    pub channel_id: Integer,
    pub user_id: Integer,
    pub message: String,
    pub chat_id: Integer,
}

impl ResponseType for Incoming {
    type Item = ();
    type Error = ();
}
