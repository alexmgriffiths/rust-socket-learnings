use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

use crate::{protocol::ServerMsg, send::send_server_msg, state::RouterState};

pub fn handle_connected_event(
    router_state: &mut RouterState,
    client_id: u64,
    out_tx: UnboundedSender<Message>,
) {
    // We could probably check serialization error here,
    // But it's kinda useless since this is a hard-coded string
    if let Err(_) = send_server_msg(
        &out_tx,
        &ServerMsg::Info {
            message: "Welcome!".to_string(),
        },
    ) {
        // Don't insert the client on failure
        return;
    }

    router_state.clients.insert(client_id, out_tx);
}
