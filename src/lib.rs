extern crate actix;
extern crate chrono;
extern crate dotenv;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate futures;
extern crate futures_state_stream;
#[cfg(test)]
extern crate rand;
extern crate telebot;
extern crate time;
extern crate tokio_core;
extern crate tokio_postgres;
extern crate tokio_timer;

mod actors;
mod error;
mod models;
mod telegram;
