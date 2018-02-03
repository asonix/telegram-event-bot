use telebot::objects::Integer;
use tokio_postgres::rows::Row;

/// Host represents a host of a scheduled Event
///
/// `user_id` is the user_id of the host
///
/// ### Relations:
/// - hosts belongs_to events (foreign_key on hosts)
///
/// ### Columns:
/// - id SERIAL
/// - user_id BIGINT
/// - event_id INTEGER REFERENCES events
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host {
    id: i32,
    user_id: Integer,
}

impl Host {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn user_id(&self) -> Integer {
        self.user_id
    }

    pub fn maybe_from_row(row: &Row, id_index: usize, user_id_index: usize) -> Option<Host> {
        let id_opt: Option<i32> = row.get(id_index);
        let user_id_opt: Option<Integer> = row.get(user_id_index);

        id_opt.and_then(|id| user_id_opt.map(|user_id| Host { id, user_id }))
    }

    pub fn from_row(row: &Row, id_index: usize, user_id_index: usize) -> Host {
        Host {
            id: row.get(id_index),
            user_id: row.get(user_id_index),
        }
    }
}

impl From<Host> for Integer {
    fn from(host: Host) -> Self {
        host.user_id
    }
}
