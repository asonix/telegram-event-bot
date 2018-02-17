#![feature(proc_macro)]

extern crate actix;
extern crate actix_web;
extern crate bcrypt;
extern crate chrono;
extern crate chrono_tz;
extern crate failure;
extern crate futures;
extern crate http;
extern crate maud;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::fmt;
use std::str::FromStr;

use actix_web::*;
use actix_web::error::ResponseError;
use actix_web::httpcodes::{HTTPCreated, HTTPOk};
use actix_web::middleware::{CookieSessionBackend, RequestSession, SessionStorage};
use chrono::{DateTime, Datelike, Timelike};
use chrono::offset::Utc;
use chrono_tz::Tz;
use failure::{Backtrace, Context, Fail, ResultExt};
use futures::Future;
use http::header;
use maud::{html, DOCTYPE};

#[derive(Debug)]
pub struct FrontendError {
    context: Context<FrontendErrorKind>,
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.context.fmt(f)
    }
}

impl Fail for FrontendError {
    fn cause(&self) -> Option<&Fail> {
        self.context.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.context.backtrace()
    }
}

impl ResponseError for FrontendError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(
            StatusCode::BAD_REQUEST,
            format!("{}, {:?}", self, self).into(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum FrontendErrorKind {
    #[fail(display = "Error generating client secret")] Generation,
    #[fail(display = "Error verifying client secret")] Verification,
    #[fail(display = "Error generating response body")] Body,
    #[fail(display = "Error generating response")] Response,
    #[fail(display = "Missing a required field")] MissingField,
    #[fail(display = "Invalid timezone")] BadTimeZone,
    #[fail(display = "Invalid year")] BadYear,
    #[fail(display = "Invalid month")] BadMonth,
    #[fail(display = "Invalid day")] BadDay,
    #[fail(display = "Invalid hour")] BadHour,
    #[fail(display = "Invalid minute")] BadMinute,
    #[fail(display = "Invalid second")] BadSecond,
    #[fail(display = "Could not find requested route")] NoRoute,
    #[fail(display = "Could not interact with session")] Session,
}

impl From<FrontendErrorKind> for FrontendError {
    fn from(e: FrontendErrorKind) -> Self {
        FrontendError {
            context: Context::new(e),
        }
    }
}

impl From<Context<FrontendErrorKind>> for FrontendError {
    fn from(e: Context<FrontendErrorKind>) -> Self {
        FrontendError { context: e }
    }
}

pub fn generate_secret(id: &str) -> Result<String, FrontendError> {
    bcrypt::hash(id, bcrypt::DEFAULT_COST)
        .context(FrontendErrorKind::Generation)
        .map_err(FrontendError::from)
}

#[derive(Clone, Debug, Eq, Fail, PartialEq)]
#[fail(display = "Missing field {}", field)]
pub struct MissingField {
    field: &'static str,
}

pub fn verify_secret(id: &str, secret: &str) -> Result<bool, FrontendError> {
    bcrypt::verify(id, secret)
        .context(FrontendErrorKind::Verification)
        .map_err(FrontendError::from)
}

fn new_form(mut req: HttpRequest) -> Result<HttpResponse, FrontendError> {
    let id = req.match_info()["secret"].to_owned();

    let option_event: Option<OptionEvent> = req.session()
        .get("option_event")
        .map_err(|_| FrontendErrorKind::Session)?;

    let submit_url = format!("/events/new/{}", id);

    let date = Utc::now().with_timezone(&Tz::US__Central);

    let years = (date.year()..date.year() + 4).collect::<Vec<_>>();

    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ].into_iter()
        .enumerate()
        .map(|(u, m)| (u as u32, m))
        .collect::<Vec<_>>();

    let days = (1..32).collect::<Vec<_>>();
    let hours = (0..24).collect::<Vec<_>>();
    let minutes = (0..60).collect::<Vec<_>>();

    let mut create_event = CreateEvent::default_from(date);

    if let Some(ref o) = option_event {
        create_event.merge(o);
    }

    let timezones = [
        Tz::US__Eastern,
        Tz::US__Central,
        Tz::US__Mountain,
        Tz::US__Pacific,
    ].into_iter()
        .map(|tz| tz.name())
        .collect::<Vec<_>>();

    let markup = html! {
        (DOCTYPE)
        html {
            head {
                title "EventBot | New Event";
            }
            body {
                section {
                    @if let Some(o) = option_event {
                        article.missing-keys {
                            h1 {
                                "Please provide the following keys"
                            }
                            ul {
                                @for key in &o.missing_keys() {
                                    li {
                                        (key)
                                    }
                                }
                            }
                        }
                    }
                    article {
                        form#event action=(submit_url) method="POST" {
                            label for="title" "Title";
                            input type="text" name="title" value=(create_event.title);

                            label for="description" "Description";
                            textarea form="event" name="description" value=(create_event.description) {
                            }

                            lable for="year" "Year";
                            select name="year" {
                                @for year in &years {
                                    @if year == &create_event.year {
                                        option value=(year) selected="true" {
                                            (year)
                                        }
                                    } @else {
                                        option value=(year) {
                                            (year)
                                        }
                                    }
                                }
                            }

                            label for="month" "Month";
                            select name="month" {
                                @for &(i, month) in &months {
                                    @if i == create_event.month {
                                        option value=(i) selected="true" {
                                            (month)
                                        }
                                    } @else {
                                        option value=(i) {
                                            (month)
                                        }
                                    }
                                }
                            }

                            label for="day" "Day";
                            select name="day" {
                                @for day in &days {
                                    @if day == &create_event.day {
                                        option value=(day) selected="true" {
                                            (day)
                                        }
                                    } @else {
                                        option value=(day) {
                                            (day)
                                        }
                                    }
                                }
                            }

                            label for="hour" "Hour";
                            select name="hour" {
                                @for hour in &hours {
                                    @if hour == &create_event.hour {
                                        option value=(hour) selected="true" {
                                            (hour)
                                        }
                                    } @else {
                                        option value=(hour) {
                                            (hour)
                                        }
                                    }
                                }
                            }

                            label for="minute" "Minute";
                            select name="minute" {
                                @for minute in &minutes {
                                    @if minute == &create_event.minute {
                                        option value=(minute) selected="true" {
                                            @if *minute < 10 {
                                                (format!("0{}", minute))
                                            } @else {
                                                (minute)
                                            }
                                        }
                                    } @else {
                                        option value=(minute) {
                                            @if *minute < 10 {
                                                (format!("0{}", minute))
                                            } @else {
                                                (minute)
                                            }
                                        }
                                    }
                                }
                            }

                            label for="timezone" "Timezone";
                            select name="timezone" {
                                @for tz in &timezones {
                                    @if tz == &create_event.timezone {
                                        option value=(tz) selected="true" {
                                            (tz)
                                        }
                                    } @else {
                                        option value=(tz) {
                                            (tz)
                                        }
                                    }
                                }
                            }

                            input type="hidden" name="secret" value=(id);
                            input type="submit" value="Submit";
                        }
                    }
                }
            }
        }
    };

    Ok(HTTPOk
        .build()
        .header(header::CONTENT_TYPE, "text/html")
        .body(markup.into_string())
        .context(FrontendErrorKind::Body)?)
}

pub struct Event {
    title: String,
    description: String,
    datetime: DateTime<Tz>,
}

impl Event {
    fn from_option(option_event: Option<OptionEvent>) -> Result<Self, FrontendError> {
        CreateEvent::from_option(option_event.ok_or(FrontendErrorKind::MissingField)?)?
            .try_to_event()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionEvent {
    title: Option<String>,
    description: Option<String>,
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,
    hour: Option<u32>,
    minute: Option<u32>,
    timezone: Option<String>,
}

impl OptionEvent {
    fn new(
        title: Option<String>,
        description: Option<String>,
        year: Option<i32>,
        month: Option<u32>,
        day: Option<u32>,
        hour: Option<u32>,
        minute: Option<u32>,
        timezone: Option<String>,
    ) -> Self {
        OptionEvent {
            title: title.and_then(|title| {
                if title.trim().len() == 0 {
                    None
                } else {
                    Some(title)
                }
            }),
            description: description.and_then(|description| {
                if description.trim().len() == 0 {
                    None
                } else {
                    Some(description)
                }
            }),
            year: year,
            month: month,
            day: day,
            hour: hour,
            minute: minute,
            timezone: timezone.and_then(|tz| if tz.trim().len() == 0 { None } else { Some(tz) }),
        }
    }

    fn missing_keys(&self) -> Vec<&'static str> {
        let mut v = Vec::new();

        if self.title.is_none() {
            v.push("title");
        }

        if self.description.is_none() {
            v.push("description");
        }

        if self.year.is_none() {
            v.push("year");
        }

        if self.month.is_none() {
            v.push("month");
        }

        if self.day.is_none() {
            v.push("day");
        }

        if self.hour.is_none() {
            v.push("hour");
        }

        if self.minute.is_none() {
            v.push("minute");
        }

        if self.timezone.is_none() {
            v.push("timezone");
        }

        v
    }
}

pub struct CreateEvent {
    title: String,
    description: String,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    timezone: String,
}

impl CreateEvent {
    fn default_from(date: DateTime<Tz>) -> Self {
        CreateEvent {
            title: "".to_owned(),
            description: "".to_owned(),
            year: date.year(),
            month: date.month() - 1,
            day: date.day() as u32,
            hour: date.hour() as u32,
            minute: date.minute() as u32,
            timezone: date.timezone().name().to_owned(),
        }
    }

    fn merge(&mut self, option_event: &OptionEvent) {
        if let Some(ref title) = option_event.title {
            self.title = title.to_owned();
        }

        if let Some(ref description) = option_event.description {
            self.description = description.to_owned();
        }

        if let Some(year) = option_event.year {
            self.year = year;
        }

        if let Some(month) = option_event.month {
            self.month = month;
        }

        if let Some(day) = option_event.day {
            self.day = day;
        }

        if let Some(hour) = option_event.hour {
            self.hour = hour;
        }

        if let Some(minute) = option_event.minute {
            self.minute = minute;
        }

        if let Some(ref timezone) = option_event.timezone {
            self.timezone = timezone.to_owned();
        }
    }

    fn from_option(option_event: OptionEvent) -> Result<Self, FrontendError> {
        let title = maybe_empty_string(maybe_field(option_event.title, "title")?, "title")?;
        let description = maybe_empty_string(
            maybe_field(option_event.description, "description")?,
            "description",
        )?;
        let year = maybe_field(option_event.year, "year")?;
        let month = maybe_field(option_event.month, "month")?;
        let day = maybe_field(option_event.day, "day")?;
        let hour = maybe_field(option_event.hour, "hour")?;
        let minute = maybe_field(option_event.minute, "minute")?;
        let timezone = maybe_field(option_event.timezone, "timezone")?;

        Ok(CreateEvent {
            title,
            description,
            year,
            month,
            day,
            hour,
            minute,
            timezone,
        })
    }

    fn try_to_event(self) -> Result<Event, FrontendError> {
        let timezone = Tz::from_str(&self.timezone).map_err(|_| FrontendErrorKind::BadTimeZone)?;

        let now = Utc::now();

        let datetime = now.with_timezone(&timezone);
        let datetime = datetime
            .with_year(self.year)
            .ok_or(FrontendErrorKind::BadYear)?
            .with_month0(self.month)
            .ok_or(FrontendErrorKind::BadMonth)?
            .with_day(self.day)
            .ok_or(FrontendErrorKind::BadDay)?
            .with_hour(self.hour)
            .ok_or(FrontendErrorKind::BadHour)?
            .with_minute(self.minute)
            .ok_or(FrontendErrorKind::BadMinute)?
            .with_second(0)
            .ok_or(FrontendErrorKind::BadSecond)?;

        Ok(Event {
            title: self.title,
            description: self.description,
            datetime: datetime,
        })
    }
}

fn maybe_field<T>(maybe: Option<T>, field: &'static str) -> Result<T, FrontendError> {
    Ok(maybe
        .ok_or(MissingField { field })
        .context(FrontendErrorKind::MissingField)?)
}

fn maybe_empty_string(s: String, field: &'static str) -> Result<String, FrontendError> {
    let s = s.trim().to_owned();

    if s.len() == 0 {
        Err(MissingField { field }
            .context(FrontendErrorKind::MissingField)
            .into())
    } else {
        Ok(s)
    }
}

fn submitted(mut req: HttpRequest) -> Box<Future<Item = HttpResponse, Error = FrontendError>> {
    Box::new(
        req.urlencoded()
            .map_err(|e| e.context(FrontendErrorKind::MissingField).into())
            .and_then(move |mut params| {
                let option_event = OptionEvent::new(
                    params.remove("title"),
                    params.remove("description"),
                    params.remove("year").and_then(|y| y.parse().ok()),
                    params.remove("month").and_then(|m| m.parse().ok()),
                    params.remove("day").and_then(|d| d.parse().ok()),
                    params.remove("hour").and_then(|h| h.parse().ok()),
                    params.remove("minute").and_then(|m| m.parse().ok()),
                    params.remove("timezone"),
                );

                req.session()
                    .set("option_event", option_event)
                    .map(move |_| req)
                    .map_err(|_| FrontendErrorKind::Session.into())
            })
            .and_then(|mut req| {
                Event::from_option(req.session().get("option_event").unwrap_or(None))
                    .and_then(|event| {
                        HTTPCreated
                            .build()
                            .header(header::CONTENT_TYPE, "text/html")
                            .body(success(event))
                            .context(FrontendErrorKind::Body)
                            .map_err(FrontendError::from)
                    })
                    .or_else(move |_| new_form(req))
            }),
    )
}

fn success(event: Event) -> String {
    let markup = html! {
        (DOCTYPE)
        html {
            head {
                title {
                    "EventBot | Event Created"
                }
            }
            body {
                section {
                    article {
                        h1 {
                            "Thanks for creating an event!"
                        }
                        h3 {
                            (event.title)
                        }
                        p {
                            (event.description)
                        }
                        p {
                            (event.datetime.to_rfc2822())
                        }
                    }
                }
            }
        }
    };

    markup.into_string()
}

pub fn run() {
    HttpServer::new(|| {
        Application::new()
            .middleware(SessionStorage::new(
                CookieSessionBackend::build(&[0; 128])
                    .secure(false)
                    .finish(),
            ))
            .resource("/events/new/{secret}", |r| {
                r.method(Method::GET).f(new_form);
                r.method(Method::POST).f(submitted);
            })
    }).bind("127.0.0.1:8000")
        .unwrap()
        .run()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
