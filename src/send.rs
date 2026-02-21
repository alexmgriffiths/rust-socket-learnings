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
) -> Result<(), SendServerMsgError> {
    if let Err(e) = sender.send(message) {
        return Err(SendServerMsgError::ClientError { error: e });
    }
    Ok(())
}

pub fn broadcast_server_msg(
    router_state: &mut RouterState,
    message: &ServerMsg,
    skip_client: Option<u64>,
) {
    // We need a list of clients who have registered.
    let broadcast_recipients = router_state
        .clients
        .iter()
        .filter(|(id, _)| Some(**id) != skip_client);

    // We don't want to potentially modify the recipeints as we loop later, so make a clone
    let mut senders: Vec<(u64, UnboundedSender<Message>)> = vec![];
    for (recipient_id, rtx) in broadcast_recipients {
        senders.push((*recipient_id, rtx.clone()));
    }

    for (id, tx) in senders {
        // NOTE:
        // This bascially loops back here to self.send_server_msg
        // We send it to router_state to deal with the client
        router_state.send_or_disconnect_server_msg(id, &tx, message);
    }
}

pub fn broadcast_ws_msg(
    router_state: &mut RouterState,
    message: Message,
    skip_client: Option<u64>,
) {
    // We need a list of clients who have registered.
    let broadcast_recipients = router_state
        .clients
        .iter()
        .filter(|(id, _)| Some(**id) != skip_client);

    // We don't want to potentially modify the recipeints as we loop later, so make a clone
    let mut senders: Vec<(u64, UnboundedSender<Message>)> = vec![];
    for (recipient_id, rtx) in broadcast_recipients {
        senders.push((*recipient_id, rtx.clone()));
    }

    for (id, tx) in senders {
        // TODO: Optimize
        // Performance can be gained here by removing ServerMsg
        // Because ServerMsg uses serde_json on every loop,
        // We could serialize once then send, but that requires a larger refactor
        router_state.send_or_disconnect_ws_msg(id, &tx, message.clone());
    }
}
