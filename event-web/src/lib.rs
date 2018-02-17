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

use actix_web::*;
use actix_web::httpcodes::{HTTPCreated, HTTPOk};
use actix_web::middleware::{CookieSessionBackend, RequestSession, SessionStorage};
use chrono::Datelike;
use chrono::offset::Utc;
use chrono_tz::Tz;
use failure::{Fail, ResultExt};
use futures::Future;
use http::header;
use maud::{html, DOCTYPE};

mod error;
mod event;

pub use error::{FrontendError, FrontendErrorKind, MissingField};
pub use event::{CreateEvent, Event, OptionEvent};

pub fn generate_secret(id: &str) -> Result<String, FrontendError> {
    bcrypt::hash(id, bcrypt::DEFAULT_COST)
        .context(FrontendErrorKind::Generation)
        .map_err(FrontendError::from)
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
                            (event.title())
                        }
                        p {
                            (event.description())
                        }
                        p {
                            (event.datetime().to_rfc2822())
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
