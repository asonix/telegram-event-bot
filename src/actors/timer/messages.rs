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

//! This module defines the messages that the Timer can receive.

use actix::Message;

use models::event::Event;

pub struct NextHour;

/// This asks the Timer to check the next hour for events that need to be scheduled for action
impl Message for NextHour {
    type Result = ();
}

/// This provides the Timer with events that need to be scheduled for action
pub struct Events {
    pub events: Vec<Event>,
}

impl Message for Events {
    type Result = ();
}

/// This notifies the Timer that the stream providing events has errored.
pub struct Shutdown;

impl Message for Shutdown {
    type Result = ();
}

/// This notifies the Timer that it should check it's stored events for pending actions
pub struct Migrate;

impl Message for Migrate {
    type Result = ();
}

/// This notifies the Timer that the Migrate stream has errored.
pub struct MigrateError;

impl Message for MigrateError {
    type Result = ();
}

/// This notifies the Timer that an event has updated.
pub struct UpdateEvent {
    pub event: Event,
}

impl Message for UpdateEvent {
    type Result = ();
}
