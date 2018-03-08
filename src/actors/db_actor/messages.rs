use actix::ResponseType;
use chrono::DateTime;
use chrono_tz::Tz;
use telebot::objects::Integer;

use error::EventError;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use models::edit_event_link::EditEventLink;
use models::event::Event;
use models::new_event_link::NewEventLink;
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NewUser {
    pub chat_id: Integer,
    pub user_id: Integer,
    pub username: String,
}

impl ResponseType for NewUser {
    type Item = User;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewRelation {
    pub chat_id: Integer,
    pub user_id: Integer,
}

impl ResponseType for NewRelation {
    type Item = ();
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewEvent {
    pub system_id: i32,
    pub title: String,
    pub description: String,
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
    pub hosts: Vec<i32>,
}

impl ResponseType for NewEvent {
    type Item = Event;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditEvent {
    pub id: i32,
    pub system_id: i32,
    pub title: String,
    pub description: String,
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
    pub hosts: Vec<i32>,
}

impl ResponseType for EditEvent {
    type Item = Event;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupEvent {
    pub event_id: i32,
}

impl ResponseType for LookupEvent {
    type Item = Event;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupEventsByUserId {
    pub user_id: Integer,
}

impl ResponseType for LookupEventsByUserId {
    type Item = Vec<Event>;
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
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupSystem {
    pub system_id: i32,
}

impl ResponseType for LookupSystem {
    type Item = ChatSystem;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupSystemByChannel(pub Integer);

impl ResponseType for LookupSystemByChannel {
    type Item = ChatSystem;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetEventsForSystem {
    pub system_id: i32,
}

impl ResponseType for GetEventsForSystem {
    type Item = Vec<Event>;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct LookupUser(pub Integer);

impl ResponseType for LookupUser {
    type Item = User;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct GetUsersWithChats;

impl ResponseType for GetUsersWithChats {
    type Item = Vec<(User, Chat)>;
    type Error = EventError;
}

#[derive(Clone, Debug)]
pub struct StoreEditEventLink {
    pub user_id: i32,
    pub event_id: i32,
    pub system_id: i32,
    pub secret: String,
}

impl ResponseType for StoreEditEventLink {
    type Item = EditEventLink;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct EditEventLinkByEventId {
    pub event_id: i32,
}

impl ResponseType for EditEventLinkByEventId {
    type Item = EditEventLink;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct DeleteEditEventLink {
    pub id: i32,
}

impl ResponseType for DeleteEditEventLink {
    type Item = ();
    type Error = EventError;
}

#[derive(Clone, Debug)]
pub struct StoreEventLink {
    pub user_id: i32,
    pub system_id: i32,
    pub secret: String,
}

impl ResponseType for StoreEventLink {
    type Item = NewEventLink;
    type Error = EventError;
}
#[derive(Clone, Copy, Debug)]
pub struct EventLinkByUserId {
    pub user_id: i32,
}

impl ResponseType for EventLinkByUserId {
    type Item = NewEventLink;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct DeleteEventLink {
    pub id: i32,
}

impl ResponseType for DeleteEventLink {
    type Item = ();
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct GetSystemsWithChats;

impl ResponseType for GetSystemsWithChats {
    type Item = Vec<(ChatSystem, Chat)>;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct RemoveUserChat(pub Integer, pub Integer);

impl ResponseType for RemoveUserChat {
    type Item = ();
    type Error = EventError;
}

#[derive(Clone, Copy, Debug)]
pub struct DeleteUserByUserId(pub Integer);

impl ResponseType for DeleteUserByUserId {
    type Item = ();
    type Error = EventError;
}
