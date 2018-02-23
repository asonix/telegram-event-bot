use actix::Address;
use chrono::{DateTime, Datelike, TimeZone, Weekday};
use chrono_tz::US::Central;
use failure::Fail;
use futures::Future;
use telebot::RcBot;
use telebot::functions::FunctionMessage;
// use telebot::objects::{InlineKeyboardButton, InlineKeyboardMarkup, Integer};

use actors::db_broker::DbBroker;
use actors::db_actor::messages::{GetChatSystemByEventId, GetEventsForSystem, LookupSystem};
use error::EventErrorKind;
use models::chat_system::ChatSystem;
use models::event::Event;

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
            .then(|msg_res| match msg_res {
                Ok(res) => res,
                Err(err) => Err(err.context(EventErrorKind::Canceled).into()),
            })
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("Don't forget! {} is starting soon!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|_| ());

        self.bot.inner.handle.spawn(fut);
    }

    fn query_events(&self, event_id: i32, system_id: i32) {
        let db = self.db.clone();
        let bot = self.bot.clone();

        let fut = self.db
            .call_fut(LookupSystem { system_id })
            .then(|msg_res| match msg_res {
                Ok(res) => res,
                Err(err) => Err(err.context(EventErrorKind::Canceled).into()),
            })
            .and_then(move |chat_system: ChatSystem| {
                db.call_fut(GetEventsForSystem { system_id })
                    .then(|msg_res| match msg_res {
                        Ok(res) => res,
                        Err(err) => Err(err.context(EventErrorKind::Canceled).into()),
                    })
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

        self.bot.inner.handle.spawn(fut.map(|_| ()).map_err(|_| ()));
    }
}

fn format_date<T>(localtime: DateTime<T>) -> String
where
    T: TimeZone,
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

    format!("{}, {} {}{}", weekday, month, localtime.day(), day)
}

/*
fn month_button(month: i32) -> InlineKeyboardButton {
    let month_name = match month {
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
        _ => "Unknown",
    };

    InlineKeyboardButton::new(month_name.to_owned()).callback_data(format!("{}", month))
}

fn day_button(day: i32) -> InlineKeyboardButton {
    InlineKeyboardButton::new(format!("{}", day)).callback_data(format!("{}", day))
}
*/
