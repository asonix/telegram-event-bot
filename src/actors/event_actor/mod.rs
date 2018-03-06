use actix::{Address, Arbiter};
use event_web::Event as FrontendEvent;
use event_web::verify_secret;
use failure::Fail;
use futures::Future;

use actors::db_broker::DbBroker;
use actors::db_actor::messages::{DeleteEventLink, EventLinkByUserId, NewEvent};
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::NewEvent as TgNewEvent;
use actors::timer::Timer;
use actors::timer::messages::Events;
use error::{EventError, EventErrorKind};
use util::flatten;

mod actor;

#[derive(Clone)]
pub struct EventActor {
    tg: Address<TelegramActor>,
    db: Address<DbBroker>,
    timer: Address<Timer>,
}

impl EventActor {
    pub fn new(tg: Address<TelegramActor>, db: Address<DbBroker>, timer: Address<Timer>) -> Self {
        EventActor { tg, db, timer }
    }

    fn new_event(&mut self, event: FrontendEvent, id: String) {
        debug!("Got event: {:?}", event);

        if let Some(index) = id.rfind('=') {
            let (base64d, user_id) = id.split_at(index);
            let base64d = base64d.to_owned();
            let user_id = user_id.trim_left_matches('=');

            if let Ok(user_id) = user_id.parse::<i32>() {
                let database = self.db.clone();

                let tg = self.tg.clone();
                let timer = self.timer.clone();

                Arbiter::handle().spawn(
                    self.db
                        .call_fut(EventLinkByUserId { user_id })
                        .then(flatten::<EventLinkByUserId>)
                        .and_then(move |nel| match verify_secret(&base64d, nel.secret()) {
                            Ok(b) => if b {
                                Ok(nel)
                            } else {
                                Err(EventError::from(EventErrorKind::Frontend))
                            },
                            Err(e) => Err(EventError::from(e.context(EventErrorKind::Frontend))),
                        })
                        .and_then(move |nel| {
                            database
                                .call_fut(DeleteEventLink { id: nel.id() })
                                .then(flatten::<DeleteEventLink>)
                                .join(
                                    database
                                        .call_fut(NewEvent {
                                            system_id: nel.system_id(),
                                            title: event.title().to_owned(),
                                            description: event.description().to_owned(),
                                            start_date: event.start_date(),
                                            end_date: event.end_date(),
                                            hosts: vec![nel.user_id()],
                                        })
                                        .then(flatten::<NewEvent>)
                                        .map(move |event| {
                                            tg.send(TgNewEvent(event.clone()));
                                            timer.send(Events {
                                                events: vec![event],
                                            });
                                        }),
                                )
                        })
                        .map(|_| ())
                        .map_err(|e| error!("Error: {:?}", e)),
                )
            }
        }
    }
}
