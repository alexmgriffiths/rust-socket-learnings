use uuid::Uuid;

use crate::commands::authenticate::handle_authenticate_command;
use crate::commands::create_conversation::handle_create_conversation_command;
use crate::commands::say::{SayError, handle_say_command};
use crate::protocol::ServerMsg;
use crate::{protocol::Command, state::RouterState};

pub fn handle_received_event(router_state: &mut RouterState, client_id: u64, command: Command) {
    let Some(tx) = router_state.connections.get(&client_id).cloned() else {
        return;
    };
    match command {
        Command::Authenticate { token } => {
            // If the user is already authenticated, ignore this command
            if router_state.connection_to_user.contains_key(&client_id) {
                router_state.send_or_disconnect_server_msg(
                    client_id,
                    &tx,
                    &ServerMsg::Error {
                        message: "REAUTH FORBIDDEN".to_string(),
                    },
                );
                return;
            }
            let user_id = match handle_authenticate_command(router_state, client_id, &token) {
                Err(_) => {
                    // TODO: Handle actual error rather than hard-coding maybe
                    router_state.send_or_disconnect_server_msg(
                        client_id,
                        &tx,
                        &ServerMsg::Error {
                            message: "AUTH FAILED".to_string(),
                        },
                    );
                    return;
                }
                Ok(user_id) => user_id,
            };
            router_state.send_or_disconnect_server_msg(
                client_id,
                &tx,
                &ServerMsg::Info {
                    message: format!("AUTH OK {user_id}"),
                },
            );
        }
        Command::CreateConversation { participant } => {
            let parsed_participant_uuid = match Uuid::try_parse(&participant) {
                Ok(p) => p,
                Err(_) => {
                    router_state.send_or_disconnect_server_msg(
                        client_id,
                        &tx,
                        &ServerMsg::Error {
                            message: "Failed to parse participant ID".to_string(),
                        },
                    );
                    return;
                }
            };

            // Get current UUID
            let current_uuid = match router_state.connection_to_user.get(&client_id) {
                Some(i) => i,
                None => {
                    router_state.send_or_disconnect_server_msg(
                        client_id,
                        &tx,
                        &ServerMsg::Error {
                            message: "You must authenticate first".to_string(),
                        },
                    );
                    return;
                }
            };

            if *current_uuid == parsed_participant_uuid {
                router_state.send_or_disconnect_server_msg(
                    client_id,
                    &tx,
                    &ServerMsg::Error {
                        message: "You cannot create a conversation with yourself".to_string(),
                    },
                );
                return;
            }
            let participants: Vec<Uuid> = vec![*current_uuid, parsed_participant_uuid];
            let new_conversation_id =
                handle_create_conversation_command(router_state, participants);
            router_state.send_or_disconnect_server_msg(
                client_id,
                &tx,
                &ServerMsg::Info {
                    message: format!("Created conversation: {new_conversation_id}"),
                },
            );
        }
        Command::Say {
            message,
            conversation_id,
        } => {
            let conversation_id: Uuid =
                match handle_say_command(router_state, client_id, &conversation_id, &message) {
                    Err(e) => {
                        let error_msg = match e {
                            SayError::InvalidMessage => "Invalid message",
                            SayError::Unauthenticated => "You must authenticate first",
                            SayError::ConversationDoesntExist => "Conversation doesn't exist",
                            SayError::InvalidConversation => "Conversation ID is invalid",
                            SayError::NotInvolved => "You are not in this conversation",
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
                    Ok(c) => c,
                };

            // We'll use one of the optimized calls since we're broadcasting
            router_state.send_server_msg_to_conversation(conversation_id, client_id, &message);
        }
    }
}
