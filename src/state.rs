use std::collections::HashMap;

use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

use crate::{
    protocol::ServerMsg,
    send::{SendServerMsgError, send_server_msg, send_ws_msg},
};

pub struct RouterState {
    pub clients: HashMap<u64, UnboundedSender<Message>>,
    pub client_names: HashMap<u64, String>,
    pub client_rev_lookup: HashMap<String, u64>,
}

impl RouterState {
    pub fn new() -> RouterState {
        return RouterState {
            clients: HashMap::new(),
            client_names: HashMap::new(),
            client_rev_lookup: HashMap::new(),
        };
    }

    pub fn disconnect_client(&mut self, client_id: u64) {
        self.clients.remove(&client_id);
        if let Some(name) = self.client_names.remove(&client_id) {
            self.client_rev_lookup.remove(&name);
        }
    }

    pub fn send_or_disconnect_server_msg(
        &mut self,
        client_id: u64,
        tx: &UnboundedSender<Message>,
        msg: &ServerMsg,
    ) {
        if let Err(e) = send_server_msg(tx, msg) {
            match e {
                SendServerMsgError::ClientError { error } => self.disconnect_client(client_id),
                SendServerMsgError::SerializationError { error } => {
                    eprintln!("Serialization error: {error}")
                }
            }
        }
    }

    pub fn send_or_disconnect_ws_msg(
        &mut self,
        client_id: u64,
        tx: &UnboundedSender<Message>,
        msg: &Message,
    ) {
        // We don't need to do much here, just account for ClientError
        // The caller will worry about SerializationError
        if send_ws_msg(tx, msg.clone()).is_err() {
            self.disconnect_client(client_id);
        }
    }

    pub fn broadcast_server_msg(&mut self, msg: &ServerMsg, skip_client: Option<u64>) {
        let raw = match serde_json::to_string(msg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Serialization error: {e}");
                return;
            }
        };

        self.broadcast_ws_msg(Message::Text(raw.into()), skip_client);
    }

    pub fn broadcast_ws_msg(&mut self, message: Message, skip_client: Option<u64>) {
        // We need a list of clients who have registered.
        let broadcast_recipients = self
            .clients
            .iter()
            .filter(|(id, _)| Some(**id) != skip_client);

        // We don't want to potentially modify the recipeints as we loop later, so make a clone
        let mut senders: Vec<(u64, UnboundedSender<Message>)> = vec![];
        for (recipient_id, rtx) in broadcast_recipients {
            senders.push((*recipient_id, rtx.clone()));
        }

        for (id, tx) in senders {
            self.send_or_disconnect_ws_msg(id, &tx, &message);
        }
    }
}
