use std::fmt::Debug;

use actix::Address;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Weekday};
use chrono_tz::US::Central;
use futures::Future;
use telebot::RcBot;
use telebot::functions::FunctionMessage;
use telebot::objects::Integer;

use actors::db_broker::DbBroker;
use actors::db_broker::messages::{GetChatSystemByEventId, GetEventsForSystem, LookupSystem};
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

    fn update_event(&self, event: Event) {
        let localtime = event.start_date().with_timezone(&Central);
        let when = format_date(localtime);

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
                        "Event Updated!\n{}\nWhen: {}\nDuration: {}\nDescription: {}",
                        event.title(),
                        when,
                        length,
                        event.description(),
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
                            .collect();

                        print_events(bot, chat_system.events_channel(), events)
                    })
            });

        self.bot
            .inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
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

fn print_events(
    bot: RcBot,
    chat_id: Integer,
    events: Vec<Event>,
) -> impl Future<Item = (), Error = EventError> {
    let events = events
        .into_iter()
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

    bot.message(chat_id, msg)
        .send()
        .map(|_| ())
        .map_err(|e| e.context(EventErrorKind::Telegram).into())
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
