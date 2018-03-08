use actix::{Address, Arbiter};
use event_web::{Event as FrontendEvent, FrontendError, FrontendErrorKind};
use event_web::verify_secret;
use failure::Fail;
use futures::{Future, IntoFuture};

use actors::db_broker::DbBroker;
use actors::db_actor::messages::{DeleteEditEventLink, DeleteEventLink, EditEvent,
                                 EditEventLinkByEventId, EventLinkByUserId, LookupEvent, NewEvent};
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::{NewEvent as TgNewEvent, UpdateEvent as TgUpdateEvent};
use actors::timer::Timer;
use actors::timer::messages::{Events, UpdateEvent};
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

    fn lookup_event(
        &mut self,
        id: String,
    ) -> impl Future<Item = FrontendEvent, Error = FrontendError> {
        let event_id = if let Some(index) = id.rfind('=') {
            let (base64d, event_id) = id.split_at(index);
            let base64d = base64d.to_owned();
            let event_id = event_id.trim_left_matches('=');

            event_id
                .parse::<i32>()
                .map(|event_id| (event_id, base64d))
                .map_err(|e| EventError::from(e.context(EventErrorKind::Permissions)))
        } else {
            Err(EventErrorKind::Permissions.into())
        };

        let database = self.db.clone();

        event_id
            .into_future()
            .and_then(move |(event_id, base64d)| {
                database
                    .call_fut(EditEventLinkByEventId { event_id })
                    .then(flatten::<EditEventLinkByEventId>)
                    .and_then(move |eel| match verify_secret(&base64d, eel.secret()) {
                        Ok(b) => if b {
                            Ok(eel)
                        } else {
                            Err(EventError::from(EventErrorKind::Frontend))
                        },
                        Err(e) => Err(EventError::from(e.context(EventErrorKind::Frontend))),
                    })
                    .and_then(move |eel| {
                        database
                            .call_fut(LookupEvent {
                                event_id: eel.event_id(),
                            })
                            .then(flatten::<LookupEvent>)
                    })
            })
            .map(|event| {
                FrontendEvent::from_parts(
                    event.title().to_owned(),
                    event.description().to_owned(),
                    event.start_date().to_owned(),
                    event.end_date().to_owned(),
                )
            })
            .map_err(|e| FrontendError::from(e.context(FrontendErrorKind::Verification)))
    }

    fn edit_event(&mut self, event: FrontendEvent, id: String) {
        debug!("Got event: {:?}", event);

        if let Some(index) = id.rfind('=') {
            let (base64d, event_id) = id.split_at(index);
            let base64d = base64d.to_owned();
            let event_id = event_id.trim_left_matches('=');

            if let Ok(event_id) = event_id.parse::<i32>() {
                let database = self.db.clone();

                let tg = self.tg.clone();
                let timer = self.timer.clone();

                Arbiter::handle().spawn(
                    self.db
                        .call_fut(EditEventLinkByEventId { event_id })
                        .then(flatten::<EditEventLinkByEventId>)
                        .and_then(move |eel| match verify_secret(&base64d, eel.secret()) {
                            Ok(b) => if b {
                                Ok(eel)
                            } else {
                                Err(EventError::from(EventErrorKind::Frontend))
                            },
                            Err(e) => Err(EventError::from(e.context(EventErrorKind::Frontend))),
                        })
                        .and_then(move |eel| {
                            database
                                .call_fut(DeleteEditEventLink { id: eel.id() })
                                .then(flatten::<DeleteEditEventLink>)
                                .join(
                                    database
                                        .call_fut(EditEvent {
                                            id: event_id,
                                            system_id: eel.system_id(),
                                            title: event.title().to_owned(),
                                            description: event.description().to_owned(),
                                            start_date: event.start_date(),
                                            end_date: event.end_date(),
                                            hosts: vec![eel.user_id()],
                                        })
                                        .then(flatten::<NewEvent>)
                                        .map(move |event| {
                                            tg.send(TgUpdateEvent(event.clone()));
                                            timer.send(UpdateEvent { event });
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
