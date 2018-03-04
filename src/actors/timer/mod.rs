use std::collections::HashSet;
use std::time::Duration;

use actix::{Address, Arbiter};
use chrono::Duration as OldDuration;
use chrono::offset::Utc;
use futures::Future;
use tokio_timer::Timer as TokioTimer;

use actors::db_actor::DbActor;
use actors::db_actor::messages::{DeleteEvent, GetEventsInRange};
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::{EventOver, NotifyEvent};
use error::EventError;
use models::event::Event;
use util::flatten;

mod actor;
pub mod messages;

pub struct Timer {
    db: Address<DbActor>,
    tg: Address<TelegramActor>,
    notification_times: HashSet<i32>,
    delete_times: HashSet<i32>,
}

impl Timer {
    fn get_next_hour(&self) -> impl Future<Item = Vec<Event>, Error = EventError> {
        let now = Utc::now();

        self.db
            .call_fut(GetEventsInRange {
                start_date: now,
                end_date: now + OldDuration::hours(1),
            })
            .then(flatten::<GetEventsInRange>)
    }

    fn set_deleters(&mut self, events: &[Event]) {
        let now = Utc::now();

        for event in events {
            let event_id = event.id();
            let system_id = event.system_id();

            if !self.delete_times.contains(&event_id) {
                self.delete_times.insert(event_id);

                let duration = event.end_date().signed_duration_since(now).num_seconds();

                let db = self.db.clone();
                let tg = self.tg.clone();

                if duration > 0 {
                    Arbiter::handle().spawn(
                        TokioTimer::default()
                            .sleep(Duration::from_secs(duration as u64))
                            .map_err(|e| error!("Error: {:?}", e))
                            .and_then(move |_| {
                                db.send(DeleteEvent { event_id });
                                tg.send(EventOver {
                                    event_id,
                                    system_id,
                                });
                                Ok(())
                            }),
                    )
                } else {
                    db.send(DeleteEvent { event_id });
                    tg.send(EventOver {
                        event_id,
                        system_id,
                    });
                }
            }
        }
    }

    fn set_notifiers(&mut self, events: Vec<Event>) {
        let now = Utc::now();

        for event in events {
            if !self.notification_times.contains(&event.id()) {
                self.notification_times.insert(event.id());

                let duration = event.start_date().signed_duration_since(now).num_seconds();

                let tg = self.tg.clone();

                if duration > 0 {
                    Arbiter::handle().spawn(
                        TokioTimer::default()
                            .sleep(Duration::from_secs(duration as u64))
                            .map_err(|e| error!("Error: {:?}", e))
                            .and_then(move |_| {
                                tg.send(NotifyEvent(event));
                                Ok(())
                            }),
                    )
                } else {
                    self.tg.send(NotifyEvent(event));
                }
            }
        }
    }
}
