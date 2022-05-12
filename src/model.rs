use heapless::Deque as SDeque;
use std::time::SystemTime;
use time::{format_description, OffsetDateTime};

#[derive(Clone)]
pub enum Connection {
    Connected(String),
    Disconnected,
}

#[derive(Clone)]
pub struct Model {
    pub ports: Vec<String>,
    pub connection: Connection,
    pub messages: SDeque<String, 8>,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            ports: Vec::new(),
            connection: Connection::Disconnected,
            messages: SDeque::default(),
        }
    }
}

impl Model {
    pub fn is_connected(self: &Self) -> bool {
        match self.connection {
            Connection::Connected(_) => true,
            Connection::Disconnected => false,
        }
    }

    pub fn message(self: &mut Self, msg: String) {
        if self.messages.is_full() {
            self.messages.pop_front();
        }

        let format = format_description::parse("[hour]:[minute]:[second]").unwrap();

        self.messages
            .push_back(format!(
                "[{}] {}",
                OffsetDateTime::from(SystemTime::now())
                    .format(&format)
                    .unwrap(),
                msg
            ))
            .ok();
    }
}
