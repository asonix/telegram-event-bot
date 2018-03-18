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

use actix::ResponseType;

use models::event::Event;

pub struct NextHour;

/// This asks the Timer to check the next hour for events that need to be scheduled for action
impl ResponseType for NextHour {
    type Item = ();
    type Error = ();
}

/// This provides the Timer with events that need to be scheduled for action
pub struct Events {
    pub events: Vec<Event>,
}

impl ResponseType for Events {
    type Item = ();
    type Error = ();
}

/// This notifies the Timer that the stream providing events has errored.
pub struct Shutdown;

impl ResponseType for Shutdown {
    type Item = ();
    type Error = ();
}

/// This notifies the Timer that it should check it's stored events for pending actions
pub struct Migrate;

impl ResponseType for Migrate {
    type Item = ();
    type Error = ();
}

/// This notifies the Timer that the Migrate stream has errored.
pub struct MigrateError;

impl ResponseType for MigrateError {
    type Item = ();
    type Error = ();
}

/// This notifies the Timer that an event has updated.
pub struct UpdateEvent {
    pub event: Event,
}

impl ResponseType for UpdateEvent {
    type Item = ();
    type Error = ();
}
