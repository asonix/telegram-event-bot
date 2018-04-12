/*
 * This file is part of Event Web
 *
 * Event Web is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Event Web is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with Event Web.  If not, see <https://www.gnu.org/licenses/>.
 */

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

use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message, Syn};
use actix::dev::{MessageResponse, ResponseChannel};
use actix_web::*;
use actix_web::http::Method;
use actix_web::server::HttpServer;
use chrono::Datelike;
use chrono::offset::Utc;
use chrono_tz::Tz;
use failure::{Fail, ResultExt};
use futures::{Future, IntoFuture};
use futures::future::Either;
use http::header;

mod error;
mod event;
mod views;

pub use error::{FrontendError, FrontendErrorKind, MissingField};
pub use event::{CreateEvent, Event, OptionEvent};
use views::{form, success};

pub type SendFuture<T, E> = Box<Future<Item = T, Error = E> + Send>;

pub struct SendFutResponse<M>
where
    M: Message,
    M::Result: Future + Send,
{
    inner: M::Result,
}

impl<M> SendFutResponse<M>
where
    M: Message,
    M::Result: Future + Send,
{
    pub fn new(inner: M::Result) -> Self {
        SendFutResponse { inner }
    }
}

impl<A, M> MessageResponse<A, M> for SendFutResponse<M>
where
    A: Actor,
    M: Message,
    M::Result: Future + Send,
{
    fn handle<R>(self, _: &mut A::Context, tx: Option<R>)
    where
        R: ResponseChannel<M>,
    {
        if let Some(tx) = tx {
            tx.send(self.inner);
        }
    }
}

#[derive(Clone)]
pub struct EventHandler<T>
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    handler: Addr<Syn, T>,
}

impl<T> EventHandler<T>
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    pub fn new(handler: Addr<Syn, T>) -> Self {
        EventHandler { handler }
    }

    pub fn notify(
        &self,
        event: Event,
        id: String,
    ) -> impl Future<Item = (), Error = FrontendError> {
        self.handler
            .send(NewEvent(event, id))
            .then(|msg_res| match msg_res {
                Ok(res) => Either::A(res),
                Err(e) => Either::B(
                    Err(FrontendError::from(e.context(FrontendErrorKind::Canceled))).into_future(),
                ),
            })
    }

    fn request_event(&self, id: String) -> impl Future<Item = Event, Error = FrontendError> {
        self.handler
            .send(LookupEvent(id))
            .then(|msg_res| match msg_res {
                Ok(res) => Either::A(res),
                Err(e) => Either::B(
                    Err(FrontendError::from(e.context(FrontendErrorKind::Canceled))).into_future(),
                ),
            })
    }

    fn edit_event(
        &self,
        event: Event,
        id: String,
    ) -> impl Future<Item = (), Error = FrontendError> {
        self.handler
            .send(EditEvent(event.clone(), id))
            .then(|msg_res| match msg_res {
                Ok(res) => Either::A(res),
                Err(e) => Either::B(
                    Err(FrontendError::from(e.context(FrontendErrorKind::Canceled))).into_future(),
                ),
            })
    }
}

pub struct NewEvent(pub Event, pub String);

impl Message for NewEvent {
    type Result = SendFuture<(), FrontendError>;
}

pub struct EditEvent(pub Event, pub String);

impl Message for EditEvent {
    type Result = SendFuture<(), FrontendError>;
}

pub struct LookupEvent(pub String);

impl Message for LookupEvent {
    type Result = SendFuture<Event, FrontendError>;
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

fn load_form(
    form_event: Option<CreateEvent>,
    form_id: String,
    form_url: String,
    form_title: &str,
    option_event: Option<OptionEvent>,
) -> HttpResponse {
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

    let mut create_event = if let Some(ce) = form_event {
        ce
    } else {
        CreateEvent::default_from(date)
    };

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

    HttpResponse::Ok()
        .header(header::CONTENT_TYPE, "text/html")
        .body(
            form(
                create_event,
                option_event,
                form_url,
                years,
                months,
                days,
                hours,
                minutes,
                timezones,
                form_id,
                form_title,
            ).into_string(),
        )
}

fn new_form<T>(req: HttpRequest<EventHandler<T>>) -> HttpResponse
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    let id = req.match_info()["secret"].to_owned();
    let submit_url = format!("/events/new/{}", id);
    load_form(None, id, submit_url, "Event Bot | New Event", None)
}

fn edit_form<T>(
    req: HttpRequest<EventHandler<T>>,
) -> Box<Future<Item = HttpResponse, Error = FrontendError>>
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    let event_handler = req.state().clone();
    let id = req.match_info()["secret"].to_owned();
    let submit_url = format!("/events/edit/{}", id);

    Box::new(event_handler.request_event(id.clone()).map(move |event| {
        load_form(
            Some(event.into()),
            id,
            submit_url,
            "Event Bot | Edit Event",
            None,
        )
    }))
}

fn updated<T>(
    req: HttpRequest<EventHandler<T>>,
) -> Box<Future<Item = HttpResponse, Error = FrontendError>>
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    let event_handler = req.state().clone();
    let id = req.match_info()["secret"].to_owned();
    let id2 = id.clone();

    Box::new(
        req.urlencoded::<HashMap<String, String>>()
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

                Event::from_option(option_event.clone())
                    .into_future()
                    .and_then(move |event| {
                        event_handler.edit_event(event.clone(), id).map(|_| {
                            HttpResponse::Created()
                                .header(header::CONTENT_TYPE, "text/html")
                                .body(success(event, "Event Bot | Updated Event").into_string())
                        })
                    })
                    .or_else(move |_| {
                        let submit_url = format!("/events/edit/{}", id2);
                        Ok(load_form(
                            None,
                            id2,
                            submit_url,
                            "Event Bot | Edit Event",
                            Some(option_event),
                        ))
                    })
            }),
    )
}

fn submitted<T>(
    req: HttpRequest<EventHandler<T>>,
) -> Box<Future<Item = HttpResponse, Error = FrontendError>>
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    let event_handler = req.state().clone();
    let id = req.match_info()["secret"].to_owned();

    Box::new(
        req.urlencoded::<HashMap<String, String>>()
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

                Event::from_option(option_event.clone())
                    .map(|event| {
                        event_handler
                            .handler
                            .do_send(NewEvent(event.clone(), id.clone()));

                        HttpResponse::Created()
                            .header(header::CONTENT_TYPE, "text/html")
                            .body(success(event, "Event Bot | Created Event").into_string())
                    })
                    .or_else(move |_| {
                        let submit_url = format!("/events/new/{}", id);
                        Ok(load_form(
                            None,
                            id,
                            submit_url,
                            "Event Bot | New Event",
                            Some(option_event),
                        ))
                    })
            }),
    )
}

pub fn build<T>(event_handler: EventHandler<T>, prefix: Option<&str>) -> App<EventHandler<T>>
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
{
    let app = App::with_state(event_handler);

    let app = if let Some(prefix) = prefix {
        app.prefix(prefix)
    } else {
        app
    };

    app.resource("/events/new/{secret}", |r| {
        r.method(Method::GET).f(new_form);
        r.method(Method::POST).f(submitted);
    }).resource("/events/edit/{secret}", |r| {
            r.method(Method::GET).f(edit_form);
            r.method(Method::POST).f(updated);
        })
        .handler("/assets/", fs::StaticFiles::new("assets/"))
}

pub fn start<T>(handler: Addr<Syn, T>, addr: &str, prefix: Option<&'static str>)
where
    T: Actor<Context = Context<T>>
        + Handler<LookupEvent>
        + Handler<NewEvent>
        + Handler<EditEvent>
        + Clone,
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
