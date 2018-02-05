use actix::ResponseType;
use telebot::objects::Integer;

use error::EventError;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use models::user::User;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChannel {
    pub channel_id: Integer,
}

impl ResponseType for NewChannel {
    type Item = ChatSystem;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewChat {
    pub channel_id: Integer,
    pub chat_id: Integer,
}

impl ResponseType for NewChat {
    type Item = Chat;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NewUser {
    pub chat_id: Integer,
    pub user_id: Integer,
}

impl ResponseType for NewUser {
    type Item = User;
    type Error = EventError;
}
