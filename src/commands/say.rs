use crate::state::RouterState;

pub enum SayError {
    Unregistered,
    InvalidMessage,
}

pub fn handle_say_command(
    router_state: &mut RouterState,
    client_id: u64,
    message: &String,
) -> Result<(), SayError> {
    // We need to make sure this user is registered
    if let None = router_state.client_names.get(&client_id) {
        return Err(SayError::Unregistered);
    }

    // The chat message is valid
    if message.is_empty() || message.len() > 256 || !message.is_ascii() {
        return Err(SayError::InvalidMessage);
    }

    // Continue which continues to the broadcast
    Ok(())
}
