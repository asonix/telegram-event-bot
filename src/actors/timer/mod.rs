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

//! This module defines the Timer functionality.
//!
//! It handles notifying telegram when events are soon, starting, and ending.

use std::collections::HashMap;

use actix::{Addr, Arbiter, Syn, Unsync};
use chrono::offset::Utc;
use chrono::{DateTime, Duration as OldDuration, Timelike};
use chrono_tz::Tz;
use futures::Future;

use actors::db_broker::messages::{DeleteEvent, GetEventsInRange};
use actors::db_broker::DbBroker;
use actors::telegram_actor::messages::{EventOver, EventSoon, EventStarted};
use actors::telegram_actor::TelegramActor;
use error::EventError;
use models::event::Event;
use util::flatten;

mod actor;
pub mod messages;

#[derive(Clone, Debug, Hash)]
enum TimerState {
    WaitingNotify,
    WaitingStart,
    WaitingEnd,
    Future,
}

pub struct Timer {
    db: Addr<Unsync, DbBroker>,
    tg: Addr<Syn, TelegramActor>,
    times: Vec<HashMap<i32, (TimerState, Event)>>,
}

impl Timer {
    pub fn new(db: Addr<Unsync, DbBroker>, tg: Addr<Syn, TelegramActor>) -> Self {
        Timer {
            db,
            tg,
            times: (0..60).map(|_| HashMap::new()).collect(),
        }
    }

    /// Notify telegram of any events starting in the next 45 minutes, if a notification has not
    /// already been sent
    fn migrate_notify(&mut self, index: usize, event: Event) {
        debug!("Moving event {} to waiting_start", event.id());

        self.notify_soon(event.clone());
        self.times[index].insert(event.id(), (TimerState::WaitingStart, event));
    }

    /// Notify telegram of any events that have started, if a notification has not already been sent
    fn migrate_start(&mut self, next_hour: DateTime<Utc>, index: usize, event: Event) {
        let end_index = event.end_date().minute() as usize;
        self.times[index].remove(&event.id());

        if next_hour > event.end_date().with_timezone(&Utc) {
            debug!("Moving event {} to waiting_end", event.id());
            self.times[end_index].insert(event.id(), (TimerState::WaitingEnd, event.clone()));
        } else {
            debug!("Moving event {} to futures", event.id());
            self.times[end_index].insert(event.id(), (TimerState::Future, event.clone()));
        }

        self.notify_now(event);
    }

    /// Store events that are happening now, but aren't ending for a while.
    fn migrate_future(&mut self, next_hour: DateTime<Utc>, index: usize, event: Event) {
        if next_hour > event.end_date().with_timezone(&Utc) {
            debug!("Moving event {} to waiting_end", event.id());
            self.times[index].insert(event.id(), (TimerState::WaitingEnd, event));
        }
    }

    /// Notify telegram when an event has ended, if it has not already done so
    fn migrate_end(&mut self, index: usize, event: Event) {
        debug!("Removing completed event {}", event.id());
        self.times[index].remove(&event.id());
        self.delete_event(event);
    }

    fn migrate_events(&mut self) {
        debug!("Migrating events");
        let now = Utc::now();
        let next_hour = now + OldDuration::hours(1);

        let index = now.minute() as usize;

        for (event_id, (state, event)) in self.times[index].clone() {
            debug!("Checking event {}", event_id);

            match state {
                TimerState::WaitingNotify => {
                    self.migrate_notify(index, event);
                }
                TimerState::WaitingStart => {
                    self.migrate_start(next_hour, index, event);
                }
                TimerState::WaitingEnd => {
                    self.migrate_end(index, event);
                }
                TimerState::Future => {
                    self.migrate_future(next_hour, index, event);
                }
            }
        }
    }

    fn get_next_hour(&self) -> impl Future<Item = Vec<Event>, Error = EventError> {
        let now = Utc::now();

        self.db
            .send(GetEventsInRange {
                start_date: (now - OldDuration::hours(1)).with_timezone(&Tz::UTC),
                end_date: (now + OldDuration::hours(1)).with_timezone(&Tz::UTC),
            })
            .then(flatten)
    }

    fn handle_events(&mut self, events: Vec<Event>) {
        let now = Utc::now();

        for event in events {
            self.new_event(event, now);
        }
    }

    /// Search all stored events for event with ID `event_id`
    fn remove_event(&mut self, event_id: i32) -> Option<(TimerState, Event)> {
        self.times
            .iter_mut()
            .find(|map| map.contains_key(&event_id))
            .and_then(|map| map.remove(&event_id))
    }

    /// Check if we're tracking the event with ID `event_id`
    fn tracking_event(&self, event_id: i32) -> bool {
        self.times.iter().any(|map| map.contains_key(&event_id))
    }

    /// Properly place and notify telegram of an updated event
    fn update_event(&mut self, event: Event) {
        self.remove_event(event.id());

        self.new_event(event, Utc::now());
    }

    /// Properly place and notify telegram of a new event
    fn new_event(&mut self, event: Event, now: DateTime<Utc>) {
        debug!("Handling event");

        if !self.tracking_event(event.id()) {
            debug!("New event!");
            let start = event.start_date().with_timezone(&Utc);
            let end = event.end_date().with_timezone(&Utc);

            let should_have_ended = now > end;
            let ending_soon = now + OldDuration::hours(1) > end;
            let should_have_started = now > start;
            let starting_soon = now + OldDuration::minutes(45) > start;
            let should_drop = now + OldDuration::hours(1) < start;

            if should_have_ended {
                debug!("Should have ended");
                // delete event
                self.delete_event(event);
            } else {
                if should_have_started {
                    debug!("Should have started");
                    // notify start
                    self.notify_now(event.clone());

                    let end_index = event.end_date().minute() as usize;

                    if ending_soon {
                        debug!("Ending soon");
                        self.times[end_index].insert(event.id(), (TimerState::WaitingEnd, event));
                    } else {
                        debug!("Not ending soon");
                        self.times[end_index].insert(event.id(), (TimerState::Future, event));
                    }
                } else if starting_soon {
                    debug!("Starting soon");
                    self.notify_soon(event.clone());

                    self.times[event.start_date().minute() as usize]
                        .insert(event.id(), (TimerState::WaitingStart, event));
                } else if !should_drop {
                    debug!("Waiting");
                    self.times[event.start_date().minute() as usize]
                        .insert(event.id(), (TimerState::WaitingNotify, event));
                }
            }
        }
    }

    fn notify_soon(&self, event: Event) {
        self.tg.do_send(EventSoon(event));
    }

    fn notify_now(&self, event: Event) {
        self.tg.do_send(EventStarted(event));
    }

    fn delete_event(&self, event: Event) {
        let tg = self.tg.clone();

        Arbiter::handle().spawn(
            self.db
                .send(DeleteEvent {
                    event_id: event.id(),
                })
                .then(flatten)
                .map(move |_| {
                    tg.do_send(EventOver(event));
                })
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }
}
