use uuid::Uuid;

use crate::state::RouterState;

pub enum SayError {
    ConversationDoesntExist,
    Unauthenticated,
    InvalidConversation,
    InvalidMessage,
    NotInvolved,
}

pub fn handle_say_command(
    router_state: &mut RouterState,
    connection_id: u64,
    conversation_id: &str,
    message: &str,
) -> Result<Uuid, SayError> {
    // We need to make sure this user is authenticated
    let Some(sender_uuid) = router_state.connection_to_user.get(&connection_id) else {
        return Err(SayError::Unauthenticated);
    };

    // The chat message is valid
    if message.is_empty() || message.len() > 256 || !message.is_ascii() {
        return Err(SayError::InvalidMessage);
    }

    // We need the senders UUID
    let parsed_conversation_id = match Uuid::try_parse(conversation_id) {
        Ok(c) => c,
        Err(_) => {
            return Err(SayError::InvalidConversation);
        }
    };

    // We need to check this user is actually part of that conversation
    let participants = match router_state.conversations.get(&parsed_conversation_id) {
        Some(c) => c.participants.clone(),
        None => {
            return Err(SayError::ConversationDoesntExist);
        }
    };

    if !participants.contains(sender_uuid) {
        return Err(SayError::NotInvolved);
    }

    // Continue which continues to the broadcast
    Ok(parsed_conversation_id)
}
