#![feature(conservative_impl_trait)]

extern crate actix;
extern crate base64;
extern crate chrono;
extern crate chrono_tz;
extern crate dotenv;
extern crate event_web;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate futures;
extern crate futures_state_stream;
#[macro_use]
extern crate log;
extern crate rand;
extern crate telebot;
extern crate time;
extern crate tokio_core;
extern crate tokio_postgres;
extern crate tokio_timer;

pub mod actors;
pub mod conn;
pub mod error;

mod models;
