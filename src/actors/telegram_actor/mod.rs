use std::collections::HashSet;
use std::fmt::Debug;

use actix::Address;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Weekday};
use chrono_tz::US::Central;
use futures::{Future, Stream};
use futures::stream::iter_ok;
use telebot::RcBot;
use telebot::functions::{FunctionGetChat, FunctionGetChatAdministrators, FunctionMessage};
use telebot::objects::{InlineKeyboardButton, InlineKeyboardMarkup, Integer};

use actors::db_broker::DbBroker;
use actors::db_actor::messages::{GetChatSystemByEventId, GetEventsForSystem, LookupSystem};
use error::{EventError, EventErrorKind};
use models::chat_system::ChatSystem;
use models::event::Event;
use util::flatten;

mod actor;
pub mod messages;

pub struct TelegramActor {
    bot: RcBot,
    db: Address<DbBroker>,
}

impl TelegramActor {
    pub fn new(bot: RcBot, db: Address<DbBroker>) -> Self {
        TelegramActor { bot, db }
    }

    fn notify_event(&self, event: Event) {
        let bot = self.bot.clone();

        let fut = self.db
            .call_fut(GetChatSystemByEventId {
                event_id: event.id(),
            })
            .then(flatten::<GetChatSystemByEventId>)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("Don't forget! {} is starting soon!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);
    }

    fn new_event(&self, event: Event) {
        let localtime = event.start_date().with_timezone(&Central);
        let when = format_date(localtime);
        let hosts = event
            .hosts()
            .iter()
            .map(|host| format!("{}", host.user_id()))
            .collect::<Vec<_>>()
            .join(", ");

        let bot = self.bot.clone();

        let fut = self.db
            .call_fut(GetChatSystemByEventId {
                event_id: event.id(),
            })
            .then(flatten::<GetChatSystemByEventId>)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!(
                        "{}\nWhen: {}\nDescription: {}\nHosts: {}",
                        event.title(),
                        when,
                        event.description(),
                        hosts
                    ),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);
    }

    fn query_events(&self, event_id: i32, system_id: i32) {
        let db = self.db.clone();
        let bot = self.bot.clone();

        let fut = self.db
            .call_fut(LookupSystem { system_id })
            .then(flatten::<LookupSystem>)
            .and_then(move |chat_system: ChatSystem| {
                db.call_fut(GetEventsForSystem { system_id })
                    .then(flatten::<GetEventsForSystem>)
                    .and_then(move |events: Vec<Event>| {
                        let events = events
                            .into_iter()
                            .filter(|event| event.id() != event_id)
                            .map(|event| {
                                // TODO: handle more than central time
                                let localtime = event.start_date().with_timezone(&Central);
                                let when = format_date(localtime);
                                let hosts = event
                                    .hosts()
                                    .iter()
                                    .map(|host| format!("{}", host.user_id()))
                                    .collect::<Vec<_>>()
                                    .join(", ");

                                format!(
                                    "{}\nWhen: {}\nDescription: {}\nHosts: {}",
                                    event.title(),
                                    when,
                                    event.description(),
                                    hosts
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n\n");

                        bot.message(
                            chat_system.events_channel(),
                            format!("Events:\n\n{}", events),
                        ).send()
                            .map_err(|e| e.context(EventErrorKind::Telegram).into())
                    })
            });

        self.bot
            .inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
    }

    fn ask_chats(&self, chats: HashSet<Integer>, chat_id: Integer) {
        let bot = self.bot.clone();
        let bot2 = bot.clone();

        let fut = iter_ok(chats)
            .and_then(move |chat_id| {
                bot.clone()
                    .get_chat(chat_id)
                    .send()
                    .map_err(|e| e.context(EventErrorKind::TelegramLookup).into())
            })
            .map(|(_, chat)| {
                InlineKeyboardButton::new(
                    chat.title
                        .unwrap_or(chat.username.unwrap_or("No title".to_owned())),
                ).callback_data(format!("{}", chat.id))
            })
            .collect()
            .and_then(move |buttons| {
                bot2.message(
                    chat_id,
                    "Which chat would you like to create an event for?".to_owned(),
                ).reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                    .send()
                    .map_err(|e| EventError::from(e.context(EventErrorKind::Telegram)))
            });

        self.bot
            .inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
    }

    fn is_admin(
        &mut self,
        channel_id: Integer,
        chat_ids: Vec<Integer>,
    ) -> impl Future<Item = Vec<Integer>, Error = EventError> {
        self.bot
            .unban_chat_administrators(channel_id)
            .send()
            .map_err(|e| EventError::from(e.context(EventErrorKind::TelegramLookup)))
            .and_then(move |(bot, admins)| {
                let channel_admins = admins
                    .into_iter()
                    .map(|admin| admin.user.id)
                    .collect::<HashSet<_>>();

                iter_ok(chat_ids)
                    .and_then(move |chat_id| {
                        bot.unban_chat_administrators(chat_id)
                            .send()
                            .map_err(|e| e.context(EventErrorKind::TelegramLookup).into())
                            .map(move |(bot, admins)| (bot, admins, chat_id))
                    })
                    .filter_map(move |(_, admins, chat_id)| {
                        if admins
                            .into_iter()
                            .any(|admin| channel_admins.contains(&admin.user.id))
                        {
                            Some(chat_id)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
    }

    fn linked(&mut self, channel_id: Integer, chat_ids: Vec<Integer>) {
        let msg = format!(
            "Linked channel '{}' to chats ({})",
            channel_id,
            chat_ids
                .into_iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<_>>()
                .join(", ")
        );

        self.bot.inner.handle.spawn(
            self.bot
                .message(channel_id, msg)
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn print_id(&mut self, chat_id: Integer) {
        self.bot.inner.handle.spawn(
            self.bot
                .message(chat_id, format!("{}", chat_id))
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn created_channel(&mut self, chat_id: Integer) {
        self.bot.inner.handle.spawn(
            self.bot
                .message(chat_id, format!("Initialized"))
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn send_url(&mut self, chat_id: Integer, url: String) {
        self.bot.inner.handle.spawn(
            self.bot
                .message(
                    chat_id,
                    format!("Use this link to create your event: {}", url),
                )
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        )
    }
}

fn format_date<T>(localtime: DateTime<T>) -> String
where
    T: TimeZone + Debug,
{
    let weekday = match localtime.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    };

    let month = match localtime.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown Month",
    };

    let day = match localtime.day() {
        1 | 21 | 31 => "st",
        2 | 22 => "nd",
        3 | 23 => "rd",
        _ => "th",
    };

    format!(
        "{}:{} {:?}, {}, {} {}{}",
        localtime.hour(),
        localtime.minute(),
        localtime.timezone(),
        weekday,
        month,
        localtime.day(),
        day
    )
}
