use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Authenticate {
        token: String,
    },
    CreateConversation {
        participant: String,
    }, // Just take the other side, we'll use the second as ourself
    Say {
        message: String,
        conversation_id: String,
    },
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Chat {
        conversation: String,
        from: String,
        message: String,
    },
    Info {
        message: String,
    },
    Error {
        message: String,
    },
}

pub enum Event {
    Connected {
        client_id: u64,
        out_tx: UnboundedSender<Message>,
    },
    Received {
        client_id: u64,
        command: Command,
    },
    Disconnected {
        client_id: u64,
    },
}

// TODO: Move to models, or it's own folder idk yet
#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

// TODO: This too
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user: UserInfo,
    pub exp: usize,
    pub iat: usize,
}
