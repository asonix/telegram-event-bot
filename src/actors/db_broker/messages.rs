//! This module defines all the messages it is possible to send to the `DbBroker` actor

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
use tokio_postgres::Connection;

/// This type notifies the DbBroker of a connection that has been created or returned
pub struct Ready {
    pub connection: Connection,
}

impl ResponseType for Ready {
    type Item = ();
    type Error = ();
}

/// This type notifies the DbBroker of a channel that should be initialized
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChannel {
    pub channel_id: Integer,
}

impl ResponseType for NewChannel {
    type Item = ChatSystem;
    type Error = EventError;
}

/// This type notifies the DbBroker of a chat that should be associated with the given channel
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChat {
    pub channel_id: Integer,
    pub chat_id: Integer,
}

impl ResponseType for NewChat {
    type Item = Chat;
    type Error = EventError;
}

/// This type notifies the DbBroker of a new user that should be associated with the given chat
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

/// This type notifies the DbBroker of a known user that should be associated with the given chat
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewRelation {
    pub chat_id: Integer,
    pub user_id: Integer,
}

impl ResponseType for NewRelation {
    type Item = ();
    type Error = EventError;
}

/// This type notifies the DbBroker that a given Channel should be deleted. Deleting a channel
/// deletes all associated chats and users as well
///
/// TODO: Make sure UsersActor has similar functionality
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeleteChannel {
    pub channel_id: Integer,
}

impl ResponseType for DeleteChannel {
    type Item = ();
    type Error = EventError;
}

/// This type notifies the DbBroker that an event should be created
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

/// This type notifies the DbBroker that the given event should be updated
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

/// This type requests events associated with the current chat
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LookupEventsByChatId {
    pub chat_id: Integer,
}

impl ResponseType for LookupEventsByChatId {
    type Item = Vec<Event>;
    type Error = EventError;
}

/// This type requests a single event by the event's ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupEvent {
    pub event_id: i32,
}

impl ResponseType for LookupEvent {
    type Item = Event;
    type Error = EventError;
}

/// This type requests events by the host's ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupEventsByUserId {
    pub user_id: Integer,
}

impl ResponseType for LookupEventsByUserId {
    type Item = Vec<Event>;
    type Error = EventError;
}

/// This type notifies the DbBroker that an event should be deleted
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeleteEvent {
    pub event_id: i32,
}

impl ResponseType for DeleteEvent {
    type Item = ();
    type Error = EventError;
}

/// This type requests Events that exist within the given time range
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetEventsInRange {
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
}

impl ResponseType for GetEventsInRange {
    type Item = Vec<Event>;
    type Error = EventError;
}

/// This type requests the ChatSystem given the system's ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupSystem {
    pub system_id: i32,
}

impl ResponseType for LookupSystem {
    type Item = ChatSystem;
    type Error = EventError;
}

/// This type requests the ChatSystem given the channel's Telegram ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupSystemByChannel(pub Integer);

impl ResponseType for LookupSystemByChannel {
    type Item = ChatSystem;
    type Error = EventError;
}

/// This type requests events associated with a ChatSystem
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetEventsForSystem {
    pub system_id: i32,
}

impl ResponseType for GetEventsForSystem {
    type Item = Vec<Event>;
    type Error = EventError;
}

/// This type requests a User given the User's Telegram ID
#[derive(Clone, Copy, Debug)]
pub struct LookupUser(pub Integer);

impl ResponseType for LookupUser {
    type Item = User;
    type Error = EventError;
}

/// This type requests all users with their associated chats
#[derive(Clone, Copy, Debug)]
pub struct GetUsersWithChats;

impl ResponseType for GetUsersWithChats {
    type Item = Vec<(User, Chat)>;
    type Error = EventError;
}

/// This type notifies the `DbBroker` that it should insert the given information as an
/// `EditEventLink`
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

/// This type requests an `EditEventLink` given it's ID
#[derive(Clone, Copy, Debug)]
pub struct LookupEditEventLink(pub i32);

impl ResponseType for LookupEditEventLink {
    type Item = EditEventLink;
    type Error = EventError;
}

/// This type notifies the `DbBroker` that an `EditEventLink` should be marked as used
#[derive(Clone, Copy, Debug)]
pub struct DeleteEditEventLink {
    pub id: i32,
}

impl ResponseType for DeleteEditEventLink {
    type Item = ();
    type Error = EventError;
}

/// This type notifies the `DbBroker` that it should insert the given information as a
/// `NewEventLink`
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

/// This type requests a `NewEventLink` by its ID
#[derive(Clone, Copy, Debug)]
pub struct LookupEventLink(pub i32);

impl ResponseType for LookupEventLink {
    type Item = NewEventLink;
    type Error = EventError;
}

/// This type notifies the `DbBroker` that a `NewEventLink` should be marked as used
#[derive(Clone, Copy, Debug)]
pub struct DeleteEventLink {
    pub id: i32,
}

impl ResponseType for DeleteEventLink {
    type Item = ();
    type Error = EventError;
}

/// This type requests every `ChatSystem` with it's associated chats
#[derive(Clone, Copy, Debug)]
pub struct GetSystemsWithChats;

impl ResponseType for GetSystemsWithChats {
    type Item = Vec<(ChatSystem, Chat)>;
    type Error = EventError;
}

/// This type notifies the `DbBroker` that it should remove the association between the User and
/// Chat given their Telegram IDs
#[derive(Clone, Copy, Debug)]
pub struct RemoveUserChat(pub Integer, pub Integer);

impl ResponseType for RemoveUserChat {
    type Item = ();
    type Error = EventError;
}

/// This type notifies the `DbBroker` that it should delete the user with the given Telegram ID
#[derive(Clone, Copy, Debug)]
pub struct DeleteUserByUserId(pub Integer);

impl ResponseType for DeleteUserByUserId {
    type Item = ();
    type Error = EventError;
}
