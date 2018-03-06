use std::collections::{HashMap, HashSet};

use actix::{Address, Arbiter};
use chrono::{DateTime, Duration as OldDuration, Timelike};
use chrono::offset::Utc;
use chrono_tz::Tz;
use futures::Future;

use actors::db_broker::DbBroker;
use actors::db_actor::messages::{DeleteEvent, GetEventsInRange};
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::{EventOver, EventSoon, EventStarted};
use error::EventError;
use models::event::Event;
use util::flatten;

mod actor;
pub mod messages;

enum TimerState {
    WaitingNotify,
    WaitingStart,
    WaitingEnd,
    Future,
}

pub struct Timer {
    db: Address<DbBroker>,
    tg: Address<TelegramActor>,
    waiting_notify: Vec<HashSet<i32>>,
    waiting_start: Vec<HashSet<i32>>,
    waiting_end: Vec<HashSet<i32>>,
    futures: Vec<HashSet<i32>>,
    states: HashMap<i32, TimerState>,
    events: HashMap<i32, Event>,
}

impl Timer {
    pub fn new(db: Address<DbBroker>, tg: Address<TelegramActor>) -> Self {
        Timer {
            db,
            tg,
            waiting_notify: (0..60).map(|_| HashSet::new()).collect(),
            waiting_start: (0..60).map(|_| HashSet::new()).collect(),
            waiting_end: (0..60).map(|_| HashSet::new()).collect(),
            futures: (0..60).map(|_| HashSet::new()).collect(),
            states: HashMap::new(),
            events: HashMap::new(),
        }
    }

    fn migrate_notifies(&mut self, now: DateTime<Utc>) {
        let notify_time = now + OldDuration::minutes(45);
        let index = notify_time.minute() as usize;

        let mut ids = self.waiting_notify[index].clone();
        self.waiting_notify[index] = HashSet::new();

        for event_id in ids.drain() {
            if let Some(event) = self.events.get(&event_id).cloned() {
                debug!("Moving event {} to waiting_start", event_id);
                self.notify_soon(event);
                self.states.insert(event_id, TimerState::WaitingStart);
                self.waiting_start[index].insert(event_id);
            } else {
                error!("Event {} is missing", event_id);
                self.waiting_notify[index].remove(&event_id);
                self.states.remove(&event_id);
            }
        }
    }

    fn migrate_starts(&mut self, now: DateTime<Utc>) {
        let index = now.minute() as usize;

        let mut ids = self.waiting_start[index].clone();
        self.waiting_start[index] = HashSet::new();

        let hour_from_now = now + OldDuration::hours(1);

        for event_id in ids.drain() {
            if let Some(event) = self.events.get(&event_id).cloned() {
                let end_index = event.end_date().minute() as usize;

                if hour_from_now > event.end_date().with_timezone(&Utc) {
                    debug!("Moving event {} to waiting_end", event_id);
                    self.states.insert(event_id, TimerState::WaitingEnd);
                    self.waiting_end[end_index].insert(event_id);
                } else {
                    debug!("Moving event {} to futures", event_id);
                    self.states.insert(event_id, TimerState::Future);
                    self.futures[end_index].insert(event_id);
                }
                self.notify_now(event);
            } else {
                error!("Event {} is missing", event_id);
                self.waiting_start[index].remove(&event_id);
                self.states.remove(&event_id);
            }
        }
    }

    fn migrate_futures(&mut self, now: DateTime<Utc>) {
        let next_hour = now + OldDuration::hours(1);
        let index = now.minute() as usize;

        for event_id in self.futures[index].clone() {
            if let Some(event) = self.events.get(&event_id).cloned() {
                if next_hour > event.end_date().with_timezone(&Utc) {
                    debug!("Moving event {} to waiting_end", event_id);
                    self.futures[index].remove(&event_id);
                    self.states.insert(event_id, TimerState::WaitingEnd);
                    self.waiting_end[index].insert(event_id);
                }
            } else {
                error!("Event {} is missing", event_id);
                self.futures[index].remove(&event_id);
                self.states.remove(&event_id);
            }
        }
    }

    fn migrate_ends(&mut self, now: DateTime<Utc>) {
        let index = now.minute() as usize;

        let mut ids = self.waiting_end[index].clone();
        self.waiting_end[index] = HashSet::new();

        for event_id in ids.drain() {
            if let Some(event) = self.events.remove(&event_id) {
                self.states.remove(&event_id);
                self.delete_event(event);
            }
        }
    }

    fn migrate_events(&mut self) {
        debug!("Migrating events");
        let now = Utc::now();

        self.migrate_futures(now);
        self.migrate_notifies(now);
        self.migrate_starts(now);
        self.migrate_ends(now);
    }

    fn get_next_hour(&self) -> impl Future<Item = Vec<Event>, Error = EventError> {
        let now = Utc::now();

        self.db
            .call_fut(GetEventsInRange {
                start_date: (now - OldDuration::hours(1)).with_timezone(&Tz::UTC),
                end_date: (now + OldDuration::hours(1)).with_timezone(&Tz::UTC),
            })
            .then(flatten::<GetEventsInRange>)
    }

    fn handle_events(&mut self, events: Vec<Event>) {
        let now = Utc::now();

        for event in events {
            debug!("Handling event");

            match self.states.get(&event.id()) {
                Some(&TimerState::WaitingNotify) => (),
                Some(&TimerState::WaitingStart) => (),
                Some(&TimerState::WaitingEnd) => (),
                Some(&TimerState::Future) => (),
                None => {
                    debug!("New event!");
                    let start = event.start_date().with_timezone(&Utc);
                    let end = event.end_date().with_timezone(&Utc);

                    let should_have_ended = now > end;
                    let ending_soon = now + OldDuration::hours(1) > end;
                    let should_have_started = now > start;
                    let starting_soon = now + OldDuration::minutes(45) > start;
                    let should_drop = now - OldDuration::hours(1) > start;

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
                                self.waiting_end[end_index].insert(event.id());
                                self.states.insert(event.id(), TimerState::WaitingEnd);
                            } else {
                                debug!("Not ending soon");
                                self.futures[end_index].insert(event.id());
                                self.states.insert(event.id(), TimerState::Future);
                            }
                        } else if starting_soon {
                            debug!("Starting soon");
                            self.waiting_start[event.start_date().minute() as usize]
                                .insert(event.id());
                            self.notify_soon(event.clone());
                        } else if !should_drop {
                            debug!("Waiting");
                            self.waiting_notify[event.start_date().minute() as usize]
                                .insert(event.id());
                            self.states.insert(event.id(), TimerState::WaitingNotify);
                        }

                        self.events.insert(event.id(), event);
                    }
                }
            }
        }
    }

    fn notify_soon(&self, event: Event) {
        self.tg.send(EventSoon(event));
    }

    fn notify_now(&self, event: Event) {
        self.tg.send(EventStarted(event));
    }

    fn delete_event(&self, event: Event) {
        let tg = self.tg.clone();

        Arbiter::handle().spawn(
            self.db
                .call_fut(DeleteEvent {
                    event_id: event.id(),
                })
                .then(flatten::<DeleteEvent>)
                .map(move |_| {
                    tg.send(EventOver(event));
                })
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }
}
