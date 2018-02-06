use actix::ResponseType;
use chrono::DateTime;
use chrono::offset::Utc;
use telebot::objects::Integer;

use error::EventError;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use models::event::Event;
use models::user::User;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChannel {
    pub channel_id: Integer,
}

impl ResponseType for NewChannel {
    type Item = ChatSystem;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChat {
    pub channel_id: Integer,
    pub chat_id: Integer,
}

impl ResponseType for NewChat {
    type Item = Chat;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewUser {
    pub chat_id: Integer,
    pub user_id: Integer,
}

impl ResponseType for NewUser {
    type Item = User;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeleteChannel {
    pub channel_id: Integer,
}

impl ResponseType for DeleteChannel {
    type Item = ();
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NewEvent {
    pub channel_id: Integer,
    pub title: String,
    pub description: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub hosts: Vec<Integer>,
}

impl ResponseType for NewEvent {
    type Item = Event;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeleteEvent {
    pub event_id: i32,
}

impl ResponseType for DeleteEvent {
    type Item = ();
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetEventsInRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

impl ResponseType for GetEventsInRange {
    type Item = Vec<Event>;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetChatSystemByEventId {
    pub event_id: i32,
}

impl ResponseType for GetChatSystemByEventId {
    type Item = ChatSystem;
    type Error = EventError;
}
