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

mod error;
mod event;
mod views;

pub use error::{FrontendError, FrontendErrorKind, MissingField};
pub use event::{CreateEvent, Event, OptionEvent};
use views::{form, success};

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

fn load_form(mut req: HttpRequest) -> Result<HttpResponse, FrontendError> {
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

    Ok(HTTPOk
        .build()
        .header(header::CONTENT_TYPE, "text/html")
        .body(form(
            create_event,
            option_event,
            submit_url,
            years,
            months,
            days,
            hours,
            minutes,
            timezones,
            id,
        ))
        .context(FrontendErrorKind::Body)?)
}

fn new_form(mut req: HttpRequest) -> Result<HttpResponse, FrontendError> {
    req.session().remove("option_event");
    load_form(req)
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
                    .or_else(move |_| load_form(req))
            }),
    )
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
