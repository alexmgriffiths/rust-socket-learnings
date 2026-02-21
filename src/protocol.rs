use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Register { name: String },
    Say { message: String },
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Chat { from: String, message: String },
    Info { message: String },
    Error { message: String },
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
