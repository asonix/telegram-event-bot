/*
 * This file is part of Telegram Event Bot.
 *
 * Copyright © 2018 Riley Trautman
 *
 * Telegram Event Bot is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Telegram Event Bot is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Telegram Event Bot.  If not, see <http://www.gnu.org/licenses/>.
 */

//! This module defines all the messages it is possible to send to the `DbBroker` actor

use actix::Message;
use chrono::DateTime;
use chrono_tz::Tz;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use error::EventError;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use models::edit_event_link::EditEventLink;
use models::event::Event;
use models::new_event_link::NewEventLink;
use models::user::User;

/// This type notifies the DbBroker of a connection that has been created or returned
pub struct Ready {
    pub connection: Connection,
}

impl Message for Ready {
    type Result = ();
}

/// This type notifies the DbBroker of a channel that should be initialized
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChannel {
    pub channel_id: Integer,
}

impl Message for NewChannel {
    type Result = Result<ChatSystem, EventError>;
}

/// This type notifies the DbBroker of a chat that should be associated with the given channel
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChat {
    pub channel_id: Integer,
    pub chat_id: Integer,
}

impl Message for NewChat {
    type Result = Result<Chat, EventError>;
}

/// This type notifies the DbBroker of a new user that should be associated with the given chat
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NewUser {
    pub chat_id: Integer,
    pub user_id: Integer,
    pub username: String,
}

impl Message for NewUser {
    type Result = Result<User, EventError>;
}

/// This type notifies the DbBroker of a known user that should be associated with the given chat
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewRelation {
    pub chat_id: Integer,
    pub user_id: Integer,
}

impl Message for NewRelation {
    type Result = Result<(), EventError>;
}

/// This type notifies the DbBroker that a given Channel should be deleted. Deleting a channel
/// deletes all associated chats and users as well
///
/// TODO: Make sure UsersActor has similar functionality
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeleteChannel {
    pub channel_id: Integer,
}

impl Message for DeleteChannel {
    type Result = Result<(), EventError>;
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

impl Message for NewEvent {
    type Result = Result<Event, EventError>;
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

impl Message for EditEvent {
    type Result = Result<Event, EventError>;
}

/// This type requests events associated with the current chat
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LookupEventsByChatId {
    pub chat_id: Integer,
}

impl Message for LookupEventsByChatId {
    type Result = Result<Vec<Event>, EventError>;
}

/// This type requests a single event by the event's ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupEvent {
    pub event_id: i32,
}

impl Message for LookupEvent {
    type Result = Result<Event, EventError>;
}

/// This type requests events by the host's ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupEventsByUserId {
    pub user_id: Integer,
}

impl Message for LookupEventsByUserId {
    type Result = Result<Vec<Event>, EventError>;
}

/// This type notifies the DbBroker that an event should be deleted
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeleteEvent {
    pub event_id: i32,
}

impl Message for DeleteEvent {
    type Result = Result<(), EventError>;
}

/// This type requests Events that exist within the given time range
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetEventsInRange {
    pub start_date: DateTime<Tz>,
    pub end_date: DateTime<Tz>,
}

impl Message for GetEventsInRange {
    type Result = Result<Vec<Event>, EventError>;
}

/// This type requests the ChatSystem given the system's ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupSystem {
    pub system_id: i32,
}

impl Message for LookupSystem {
    type Result = Result<ChatSystem, EventError>;
}

pub struct LookupSystemWithChats {
    pub system_id: i32,
}

impl Message for LookupSystemWithChats {
    type Result = Result<(ChatSystem, Vec<Integer>), EventError>;
}

/// This type requests the ChatSystem given the channel's Telegram ID
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LookupSystemByChannel(pub Integer);

impl Message for LookupSystemByChannel {
    type Result = Result<ChatSystem, EventError>;
}

/// This type requests events associated with a ChatSystem
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GetEventsForSystem {
    pub system_id: i32,
}

impl Message for GetEventsForSystem {
    type Result = Result<Vec<Event>, EventError>;
}

/// This type requests a User given the User's Telegram ID
#[derive(Clone, Copy, Debug)]
pub struct LookupUser(pub Integer);

impl Message for LookupUser {
    type Result = Result<User, EventError>;
}

/// This type requests all users with their associated chats
#[derive(Clone, Copy, Debug)]
pub struct GetUsersWithChats;

impl Message for GetUsersWithChats {
    type Result = Result<Vec<(User, Chat)>, EventError>;
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

impl Message for StoreEditEventLink {
    type Result = Result<EditEventLink, EventError>;
}

/// This type requests an `EditEventLink` given it's ID
#[derive(Clone, Copy, Debug)]
pub struct LookupEditEventLink(pub i32);

impl Message for LookupEditEventLink {
    type Result = Result<EditEventLink, EventError>;
}

/// This type notifies the `DbBroker` that an `EditEventLink` should be marked as used
#[derive(Clone, Copy, Debug)]
pub struct DeleteEditEventLink {
    pub id: i32,
}

impl Message for DeleteEditEventLink {
    type Result = Result<(), EventError>;
}

/// This type notifies the `DbBroker` that it should insert the given information as a
/// `NewEventLink`
#[derive(Clone, Debug)]
pub struct StoreEventLink {
    pub user_id: i32,
    pub system_id: i32,
    pub secret: String,
}

impl Message for StoreEventLink {
    type Result = Result<NewEventLink, EventError>;
}

/// This type requests a `NewEventLink` by its ID
#[derive(Clone, Copy, Debug)]
pub struct LookupEventLink(pub i32);

impl Message for LookupEventLink {
    type Result = Result<NewEventLink, EventError>;
}

/// This type notifies the `DbBroker` that a `NewEventLink` should be marked as used
#[derive(Clone, Copy, Debug)]
pub struct DeleteEventLink {
    pub id: i32,
}

impl Message for DeleteEventLink {
    type Result = Result<(), EventError>;
}

/// This type requests every `ChatSystem` with it's associated chats
#[derive(Clone, Copy, Debug)]
pub struct GetSystemsWithChats;

impl Message for GetSystemsWithChats {
    type Result = Result<Vec<(ChatSystem, Chat)>, EventError>;
}

/// This type notifies the `DbBroker` that it should remove the association between the User and
/// Chat given their Telegram IDs
#[derive(Clone, Copy, Debug)]
pub struct RemoveUserChat(pub Integer, pub Integer);

impl Message for RemoveUserChat {
    type Result = Result<(), EventError>;
}

/// This type notifies the `DbBroker` that it should delete the user with the given Telegram ID
#[derive(Clone, Copy, Debug)]
pub struct DeleteUserByUserId(pub Integer);

impl Message for DeleteUserByUserId {
    type Result = Result<(), EventError>;
}
