use actix::{Address, Arbiter};
use chrono::offset::Utc;
use event_web::Event as FrontendEvent;
use event_web::verify_secret;
use failure::Fail;
use futures::Future;

use actors::db_broker::DbBroker;
use actors::db_actor::messages::{DeleteEventLink, EventLinkByUserId, NewEvent};
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::NewEvent as TgNewEvent;
use error::{EventError, EventErrorKind};

#[derive(Clone)]
pub struct EventActor {
    tg: Address<TelegramActor>,
    db: Address<DbBroker>,
}

impl EventActor {
    pub fn new(tg: Address<TelegramActor>, db: Address<DbBroker>) -> Self {
        EventActor { tg, db }
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

                Arbiter::handle().spawn(
                    self.db
                        .call_fut(EventLinkByUserId { user_id })
                        .then(|msg_res| match msg_res {
                            Ok(res) => res,
                            Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                        })
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
                                .then(|msg_res| match msg_res {
                                    Ok(res) => res,
                                    Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                                })
                                .join(
                                    database
                                        .call_fut(NewEvent {
                                            system_id: nel.system_id(),
                                            title: event.title().to_owned(),
                                            description: event.description().to_owned(),
                                            start_date: event.start_date().with_timezone(&Utc),
                                            end_date: event.end_date().with_timezone(&Utc),
                                            hosts: vec![nel.user_id()],
                                        })
                                        .then(|msg_res| match msg_res {
                                            Ok(res) => res,
                                            Err(e) => {
                                                Err(e.context(EventErrorKind::Canceled).into())
                                            }
                                        })
                                        .map(move |event| tg.send(TgNewEvent(event))),
                                )
                        })
                        .map(|_| ())
                        .map_err(|e| error!("Error: {:?}", e)),
                )
            }
        }
    }
}

mod actor {
    use actix::{Actor, Context, Handler};
    use event_web::NewEvent;

    use super::EventActor;

    impl Actor for EventActor {
        type Context = Context<Self>;
    }

    impl Handler<NewEvent> for EventActor {
        type Result = ();

        fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
            self.new_event(msg.0, msg.1);
        }
    }
}
