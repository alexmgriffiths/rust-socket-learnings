use std::collections::HashMap;

use jsonwebtoken::DecodingKey;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::{
    conversation::Conversation,
    protocol::ServerMsg,
    send::{SendServerMsgError, send_server_msg},
};

pub struct RouterState {
    pub decoding_key: DecodingKey,
    pub connections: HashMap<u64, UnboundedSender<Message>>,
    pub connection_to_user: HashMap<u64, Uuid>,
    //pub users: HashMap<Uuid, UserInfo>,
    pub conversations: HashMap<Uuid, Conversation>,
}

impl RouterState {
    pub fn new(decoding_key: DecodingKey) -> RouterState {
        RouterState {
            decoding_key,
            connections: HashMap::new(),
            connection_to_user: HashMap::new(),
            //users: HashMap::new(),
            conversations: HashMap::new(),
        }
    }

    pub fn disconnect_client(&mut self, client_id: u64) {
        self.connections.remove(&client_id);
        self.connection_to_user.remove(&client_id);

        // TODO: Consider
        // We may want to also drop the users info if they have no conversations, and potentially the conversations, but I don't want to do that yet.
        // Here's where things get tricky, what if the user is connected on two devices? What happens?
        // If we try to send a disconnect message to their conversations, what happens if they disconnect from one device but not the second?
        // We probably don't want to send a disconnect message if that happens...
    }

    pub fn send_or_disconnect_server_msg(
        &mut self,
        client_id: u64,
        tx: &UnboundedSender<Message>,
        msg: &ServerMsg,
    ) {
        if let Err(e) = send_server_msg(tx, msg) {
            match e {
                SendServerMsgError::ClientError => self.disconnect_client(client_id),
                SendServerMsgError::SerializationError { error } => {
                    eprintln!("Serialization error: {error}")
                }
            }
        }
    }

    pub fn send_server_msg_to_conversation(
        &mut self,
        conversation_id: Uuid,
        sender_connection_id: u64,
        message: &str,
    ) {
        // Get the original sender ID
        let original_sender_id = match self.connection_to_user.get(&sender_connection_id) {
            Some(s) => *s,
            None => return,
        };

        let participants = match self.conversations.get(&conversation_id) {
            Some(c) => c.participants.clone(),
            None => return,
        };

        let mut senders: Vec<(u64, UnboundedSender<Message>)> = vec![];
        for (connection_id, user_id) in &self.connection_to_user {
            if participants.contains(user_id) {
                if let Some(sender) = self.connections.get(connection_id) {
                    senders.push((*connection_id, sender.clone()));
                }
            }
        }

        for (cid, tx) in senders {
            self.send_or_disconnect_server_msg(
                cid,
                &tx,
                &ServerMsg::Chat {
                    conversation: conversation_id.to_string(),
                    from: original_sender_id.to_string(),
                    message: message.to_string(),
                },
            );
        }
    }
}
