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

use actix::{Actor, Handler, ResponseType, SyncAddress};
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

#[derive(Clone)]
pub struct EventHandler<T>
where
    T: Actor + Handler<NewEvent> + Clone,
{
    handler: SyncAddress<T>,
}

impl<T> EventHandler<T>
where
    T: Actor + Handler<NewEvent> + Clone,
{
    pub fn new(handler: SyncAddress<T>) -> Self {
        EventHandler { handler }
    }

    pub fn notify(&self, event: Event, id: String) {
        self.handler.send(NewEvent(event, id));
    }
}

pub struct NewEvent(pub Event, pub String);

impl ResponseType for NewEvent {
    type Item = ();
    type Error = ();
}

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

fn load_form<T>(mut req: HttpRequest<EventHandler<T>>) -> Result<HttpResponse, FrontendError>
where
    T: Actor + Handler<NewEvent> + Clone,
{
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
        .body(
            form(
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
            ).into_string(),
        )
        .context(FrontendErrorKind::Body)?)
}

fn new_form<T>(mut req: HttpRequest<EventHandler<T>>) -> Result<HttpResponse, FrontendError>
where
    T: Actor + Handler<NewEvent> + Clone,
{
    req.session().remove("option_event");
    load_form(req)
}

fn submitted<T>(
    mut req: HttpRequest<EventHandler<T>>,
) -> Box<Future<Item = HttpResponse, Error = FrontendError>>
where
    T: Actor + Handler<NewEvent> + Clone,
{
    let event_handler = req.state().clone();
    let id = req.match_info()["secret"].to_owned();

    Box::new(
        req.urlencoded()
            .map_err(|e| e.context(FrontendErrorKind::MissingField).into())
            .and_then(move |mut params| {
                let option_event = OptionEvent::new(
                    params.remove("title"),
                    params.remove("description"),
                    params.remove("start_year").and_then(|y| y.parse().ok()),
                    params.remove("start_month").and_then(|m| m.parse().ok()),
                    params.remove("start_day").and_then(|d| d.parse().ok()),
                    params.remove("start_hour").and_then(|h| h.parse().ok()),
                    params.remove("start_minute").and_then(|m| m.parse().ok()),
                    params.remove("end_year").and_then(|y| y.parse().ok()),
                    params.remove("end_month").and_then(|m| m.parse().ok()),
                    params.remove("end_day").and_then(|d| d.parse().ok()),
                    params.remove("end_hour").and_then(|h| h.parse().ok()),
                    params.remove("end_minute").and_then(|m| m.parse().ok()),
                    params.remove("timezone"),
                );

                req.session()
                    .set("option_event", option_event)
                    .map(move |_| req)
                    .map_err(|_| FrontendErrorKind::Session.into())
            })
            .and_then(move |mut req| {
                Event::from_option(req.session().get("option_event").unwrap_or(None))
                    .and_then(|event| {
                        event_handler.handler.send(NewEvent(event.clone(), id));

                        HTTPCreated
                            .build()
                            .header(header::CONTENT_TYPE, "text/html")
                            .body(success(event).into_string())
                            .context(FrontendErrorKind::Body)
                            .map_err(FrontendError::from)
                    })
                    .or_else(move |_| load_form(req))
            }),
    )
}

pub fn build<T>(
    event_handler: EventHandler<T>,
    prefix: Option<&str>,
) -> Application<EventHandler<T>>
where
    T: Actor + Handler<NewEvent> + Clone,
{
    let app = Application::with_state(event_handler);

    let app = if let Some(prefix) = prefix {
        app.prefix(prefix)
    } else {
        app
    };

    app.middleware(SessionStorage::new(
        CookieSessionBackend::build(&[0; 1024])
            .secure(false)
            .finish(),
    )).resource("/events/new/{secret}", |r| {
        r.method(Method::GET).f(new_form);
        r.method(Method::POST).f(submitted);
    }).handler("/assets/", fs::StaticFiles::new("assets/", true))
}

pub fn start<T>(handler: SyncAddress<T>, addr: &str, prefix: Option<&'static str>)
where
    T: Actor + Handler<NewEvent> + Clone,
{
    HttpServer::new(move || build(EventHandler::new(handler.clone()), prefix))
        .bind(addr)
        .unwrap()
        .start();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
