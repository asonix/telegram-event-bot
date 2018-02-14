use actix::ResponseType;
use telebot::objects::Update;
use telebot::RcBot;

pub struct TgUpdate {
    pub bot: RcBot,
    pub update: Update,
}

impl ResponseType for TgUpdate {
    type Item = ();
    type Error = ();
}
