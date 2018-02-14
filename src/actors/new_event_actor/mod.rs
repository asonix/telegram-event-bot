use std::collections::HashMap;

use actix::Address;
use chrono::DateTime;
use chrono::offset::Utc;
use failure::Fail;
use futures::{Future, IntoFuture};
use telebot::objects::Integer;

use actors::db_broker::DbBroker;
use actors::db_actor::messages::NewEvent;
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::*;
use error::{EventError, EventErrorKind};
use models::event::Event;
use models::user::User;

mod actor;
mod event_stages;
pub mod messages;

use self::event_stages::*;

pub struct NewEventActor {
    titles: HashMap<Integer, Title>,
    with_descriptions: HashMap<Integer, WithDescription>,
    with_dates: HashMap<Integer, WithDate>,
    with_ends: HashMap<Integer, WithEnd>,
    with_hosts: HashMap<Integer, WithHosts>,
    db_broker: Address<DbBroker>,
    tg: Address<TelegramActor>,
}

impl NewEventActor {
    pub fn new(db_broker: Address<DbBroker>, tg: Address<TelegramActor>) -> Self {
        NewEventActor {
            titles: HashMap::new(),
            with_descriptions: HashMap::new(),
            with_dates: HashMap::new(),
            with_ends: HashMap::new(),
            with_hosts: HashMap::new(),
            db_broker: db_broker,
            tg: tg,
        }
    }

    fn new_title(
        &mut self,
        user_id: Integer,
        title: String,
        channel_id: Integer,
        chat_id: Integer,
    ) {
        self.titles.insert(user_id, Title { title, channel_id });

        self.tg.send(AnswerTitle { chat_id });
    }

    fn add_description(
        &mut self,
        user_id: Integer,
        description: String,
        chat_id: Integer,
    ) -> Result<(), EventError> {
        let title = self.titles
            .remove(&user_id)
            .ok_or(EventErrorKind::MissingEvent)
            .map_err(|e| {
                self.tg.send(FailedAnswerDescription { chat_id });

                e
            })?;

        self.with_descriptions
            .insert(user_id, title.add_description(description));

        self.tg.send(AnswerDescription { chat_id });

        Ok(())
    }

    fn add_date(
        &mut self,
        user_id: Integer,
        start_date: DateTime<Utc>,
        chat_id: Integer,
    ) -> Result<(), EventError> {
        let with_description = self.with_descriptions
            .remove(&user_id)
            .ok_or(EventErrorKind::MissingEvent)
            .map_err(|e| {
                self.tg.send(FailedAnswerDate { chat_id });

                e
            })?;

        self.with_dates
            .insert(user_id, with_description.add_date(start_date));

        self.tg.send(AnswerDate { chat_id });

        Ok(())
    }

    fn add_end(
        &mut self,
        user_id: Integer,
        end_date: DateTime<Utc>,
        chat_id: Integer,
    ) -> Result<(), EventError> {
        let with_date = self.with_dates
            .remove(&user_id)
            .ok_or(EventErrorKind::MissingEvent)
            .map_err(|e| {
                self.tg.send(FailedAnswerEnd { chat_id });

                e
            })?;

        self.with_ends.insert(user_id, with_date.add_end(end_date));

        self.tg.send(AnswerEnd { chat_id });

        Ok(())
    }

    fn add_host(
        &mut self,
        user_id: Integer,
        host: User,
        chat_id: Integer,
    ) -> Result<(), EventError> {
        if let Some(with_end) = self.with_ends.remove(&user_id) {
            self.with_hosts.insert(user_id, with_end.add_host(host));

            self.tg.send(AnswerHost { chat_id });

            Ok(())
        } else if let Some(with_hosts) = self.with_hosts.get_mut(&user_id) {
            with_hosts.add_host(host);

            self.tg.send(AnswerHost { chat_id });

            Ok(())
        } else {
            self.tg.send(FailedAnswerHost { chat_id });

            Err(EventErrorKind::MissingEvent.into())
        }
    }

    fn finalize(
        &mut self,
        user_id: Integer,
        chat_id: Integer,
    ) -> Box<Future<Item = Event, Error = EventError>> {
        let db_broker = self.db_broker.clone();
        let tg = self.tg.clone();

        Box::new(
            self.with_hosts
                .remove(&user_id)
                .ok_or(EventErrorKind::MissingEvent.into())
                .into_future()
                .and_then(move |with_hosts| {
                    let new_event: NewEvent = with_hosts.into();

                    db_broker.call_fut(new_event).then(|msg_res| match msg_res {
                        Ok(res) => res,
                        Err(err) => Err(err.context(EventErrorKind::Canceled).into()),
                    })
                })
                .then(move |res| match res {
                    Ok(event) => {
                        tg.send(AnswerFinalize { chat_id });

                        Ok(event)
                    }
                    Err(err) => {
                        tg.send(FailedAnswerFinalize { chat_id });

                        Err(err)
                    }
                }),
        )
    }
}
