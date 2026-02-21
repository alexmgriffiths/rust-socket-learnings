use uuid::Uuid;

use crate::{conversation::Conversation, state::RouterState};

pub fn handle_create_conversation_command(
    router_state: &mut RouterState,
    participants: Vec<Uuid>,
) -> Uuid {
    // TODO: Conversation De-duping
    let new_conversation = Conversation::new(participants);
    let conversation_id = new_conversation.id;
    router_state
        .conversations
        .insert(conversation_id, new_conversation);
    conversation_id
}
