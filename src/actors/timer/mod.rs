use std::collections::HashSet;
use std::time::Duration;

use actix::{Actor, Address, Arbiter, Context};
use chrono::{DateTime, Duration as OldDuration};
use chrono::offset::Utc;
use failure::Fail;
use futures::Future;
use telebot::objects::Integer;
use tokio_timer::{Sleep, Timer as TokioTimer};

use actors::db_actor::DbActor;
use actors::db_actor::messages::GetEventsInRange;
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::NotifyEvent;
use error::{EventError, EventErrorKind};
use models::event::Event;

// mod actor;
// pub mod messages;

pub struct Timer {
    db: Address<DbActor>,
    tg: Address<TelegramActor>,
    notification_times: HashSet<i32>,
    delete_times: HashSet<i32>,
}

impl Actor for Timer {
    type Context = Context<Self>;
}

impl Timer {
    fn get_next_hour(&self) -> Box<Future<Item = Vec<Event>, Error = EventError>> {
        let now = Utc::now();

        Box::new(
            self.db
                .call_fut(GetEventsInRange {
                    start_date: now,
                    end_date: now + OldDuration::hours(1),
                })
                .then(|result| match result {
                    Ok(res) => res,
                    Err(e) => Err(e.context(EventErrorKind::Cancelled).into()),
                }),
        )
    }

    fn set_notifiers(&mut self, events: Vec<Event>) {
        let now = Utc::now();

        for event in events {
            if !self.notification_times.contains(&event.id()) {
                let duration = event.start_date().signed_duration_since(now).num_seconds();

                let tg = self.tg.clone();

                if duration > 0 {
                    Arbiter::handle().spawn(
                        TokioTimer::default()
                            .sleep(Duration::from_secs(duration as u64))
                            .map_err(|_| ())
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
