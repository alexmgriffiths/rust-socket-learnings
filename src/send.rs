use serde_json::Error as SerdeError;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::ServerMsg;
use crate::state::RouterState;

// We split these so the handlers can either deal with SE, or disconnect
// Dead client on ClientError
#[derive(Debug)]
pub enum SendServerMsgError {
    SerializationError { error: SerdeError },
    ClientError { error: SendError<Message> },
}

pub fn send_server_msg(
    sender: &UnboundedSender<Message>,
    message: &ServerMsg,
) -> Result<(), SendServerMsgError> {
    let parsed_message = serde_json::to_string(message);
    match parsed_message {
        Ok(s) => {
            if let Err(e) = sender.send(Message::Text(s.into())) {
                return Result::Err(SendServerMsgError::ClientError { error: e });
            }
            Ok(())
        }
        Err(e) => Err(SendServerMsgError::SerializationError { error: e }),
    }
}

pub fn send_ws_msg(
    sender: &UnboundedSender<Message>,
    message: Message,
) -> Result<(), SendError<Message>> {
    if let Err(e) = sender.send(message) {
        return Err(e);
    }
    Ok(())
}
