use serde_json::Error as SerdeError;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::ServerMsg;

// We split these so the handlers can either deal with SE, or disconnect
// Dead client on ClientError
#[derive(Debug)]
pub enum SendServerMsgError {
    SerializationError { error: SerdeError },
    ClientError,
}

pub fn send_server_msg(
    sender: &UnboundedSender<Message>,
    message: &ServerMsg,
) -> Result<(), SendServerMsgError> {
    let parsed_message = serde_json::to_string(message);
    match parsed_message {
        Ok(s) => {
            if sender.send(Message::Text(s.into())).is_err() {
                return Err(SendServerMsgError::ClientError);
            }
            Ok(())
        }
        Err(e) => Err(SendServerMsgError::SerializationError { error: e }),
    }
}
