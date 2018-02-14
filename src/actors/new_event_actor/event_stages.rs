use chrono::DateTime;
use chrono::offset::Utc;
use telebot::objects::Integer;

use actors::db_actor::messages::NewEvent;
use models::user::User;

pub struct Title {
    pub channel_id: Integer,
    pub title: String,
}

impl Title {
    pub fn add_description(self, description: String) -> WithDescription {
        WithDescription {
            prev: self,
            description: description,
        }
    }
}

pub struct WithDescription {
    prev: Title,
    description: String,
}

impl WithDescription {
    pub fn add_date(self, start_date: DateTime<Utc>) -> WithDate {
        WithDate {
            prev: self,
            start_date: start_date,
        }
    }
}

pub struct WithDate {
    prev: WithDescription,
    start_date: DateTime<Utc>,
}

impl WithDate {
    pub fn add_end(self, end_date: DateTime<Utc>) -> WithEnd {
        WithEnd {
            prev: self,
            end_date: end_date,
        }
    }
}

pub struct WithEnd {
    prev: WithDate,
    end_date: DateTime<Utc>,
}

impl WithEnd {
    pub fn add_host(self, host: User) -> WithHosts {
        WithHosts {
            prev: self,
            hosts: vec![host],
        }
    }
}

pub struct WithHosts {
    prev: WithEnd,
    hosts: Vec<User>,
}

impl WithHosts {
    pub fn add_host(&mut self, host: User) {
        self.hosts.push(host);
    }
}

impl From<WithHosts> for NewEvent {
    fn from(with_hosts: WithHosts) -> Self {
        NewEvent {
            channel_id: with_hosts.prev.prev.prev.prev.channel_id,
            title: with_hosts.prev.prev.prev.prev.title,
            description: with_hosts.prev.prev.prev.description,
            start_date: with_hosts.prev.prev.start_date,
            end_date: with_hosts.prev.end_date,
            hosts: with_hosts
                .hosts
                .into_iter()
                .map(|user| user.user_id())
                .collect(),
        }
    }
}
