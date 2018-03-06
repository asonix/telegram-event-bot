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

    fn event_soon(&self, event: Event) {
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

    fn event_over(&self, event: Event) {
        let bot = self.bot.clone();

        let id = event.id();
        let system_id = event.system_id();

        let fut = self.db
            .call_fut(GetChatSystemByEventId {
                event_id: event.id(),
            })
            .then(flatten::<GetChatSystemByEventId>)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("{} has ended!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);

        self.query_events(id, system_id);
    }

    fn event_started(&self, event: Event) {
        let bot = self.bot.clone();

        let fut = self.db
            .call_fut(GetChatSystemByEventId {
                event_id: event.id(),
            })
            .then(flatten::<GetChatSystemByEventId>)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("{} has started!", event.title()),
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
            .map(|host| format!("@{}", host.username()))
            .collect::<Vec<_>>()
            .join(", ");

        let length = format_duration(&event);

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
                        "New Event!\n{}\nWhen: {}\nDuration: {}\nDescription: {}\nHosts: {}",
                        event.title(),
                        when,
                        length,
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
            .map_err(|e| {
                error!("LookupSystem");
                e
            })
            .and_then(move |chat_system: ChatSystem| {
                db.call_fut(GetEventsForSystem { system_id })
                    .then(flatten::<GetEventsForSystem>)
                    .map_err(|e| {
                        error!("GetEventsForSystem");
                        e
                    })
                    .and_then(move |events: Vec<Event>| {
                        let events = events
                            .into_iter()
                            .filter(|event| event.id() != event_id)
                            .map(|event| {
                                let localtime = event.start_date().with_timezone(&Central);
                                let when = format_date(localtime);
                                let duration = format_duration(&event);
                                let hosts = event
                                    .hosts()
                                    .iter()
                                    .map(|host| format!("@{}", host.username()))
                                    .collect::<Vec<_>>()
                                    .join(", ");

                                format!(
                                    "{}\nWhen: {}\nDuration: {}\nDescription: {}\nHosts: {}",
                                    event.title(),
                                    when,
                                    duration,
                                    event.description(),
                                    hosts
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n\n");

                        let msg = if events.len() > 0 {
                            format!("Upcoming Events:\n\n{}", events)
                        } else {
                            "No upcoming events".to_owned()
                        };

                        bot.message(chat_system.events_channel(), msg)
                            .send()
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

fn format_duration(event: &Event) -> String {
    let duration = event
        .end_date()
        .signed_duration_since(event.start_date().clone());

    if duration.num_weeks() > 0 {
        format!("{} Weeks", duration.num_weeks())
    } else if duration.num_days() > 0 {
        format!("{} Days", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} Hours", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} Minutes", duration.num_minutes())
    } else {
        "No time".to_owned()
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

    let minute = if localtime.minute() > 9 {
        format!("{}", localtime.minute())
    } else {
        format!("0{}", localtime.minute())
    };

    format!(
        "{}:{} {:?}, {}, {} {}{}",
        localtime.hour(),
        minute,
        localtime.timezone(),
        weekday,
        month,
        localtime.day(),
        day
    )
}
