use tokio_tungstenite::tungstenite::Message;

use crate::commands::register::{self, RegisterError};
use crate::commands::say::{self, SayError};
use crate::protocol::ServerMsg;
use crate::send::broadcast_ws_msg;
use crate::{protocol::Command, state::RouterState};

pub fn handle_received_event(router_state: &mut RouterState, client_id: u64, command: Command) {
    let Some(tx) = router_state.clients.get(&client_id).cloned() else {
        return;
    };
    match command {
        Command::Register { name } => {
            let register_response =
                register::handle_register_command(router_state, client_id, &name);
            if let Err(e) = register_response {
                let error_msg = match e {
                    RegisterError::InvalidName => "Invalid name",
                    RegisterError::AlreadyRegistered => "Already registered",
                    RegisterError::UsernameTaken => "Username taken",
                };
                router_state.send_or_disconnect_server_msg(
                    client_id,
                    &tx,
                    &ServerMsg::Error {
                        message: error_msg.to_string(),
                    },
                );
                return;
            }

            router_state.send_or_disconnect_server_msg(
                client_id,
                &tx,
                &ServerMsg::Info {
                    message: format!("Registered as {name}").to_string(),
                },
            );
        }
        Command::Say { message } => {
            let say_response = say::handle_say_command(router_state, client_id, &message);
            if let Err(e) = say_response {
                let error_msg = match e {
                    SayError::InvalidMessage => "Invalid message",
                    SayError::Unregistered => "You must register first",
                };
                router_state.send_or_disconnect_server_msg(
                    client_id,
                    &tx,
                    &ServerMsg::Error {
                        message: error_msg.to_string(),
                    },
                );
                return;
            }

            // Get registered name of client
            // I think unwrapping is okay since the above checks that they are registered
            let sender_name = router_state.client_names.get(&client_id).unwrap();

            // We'll use one of the optimized calls since we're broadcasting
            let message_to_broadcast_raw = match serde_json::to_string(&ServerMsg::Chat {
                from: sender_name.clone(),
                message,
            }) {
                Err(e) => {
                    eprintln!("failed to serialize broadcast: {e}");
                    return;
                }
                Ok(v) => v,
            };

            let message_to_send = Message::Text(message_to_broadcast_raw.into());
            broadcast_ws_msg(router_state, message_to_send, Some(client_id));
        }
    }
}
