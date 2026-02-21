use crate::state::RouterState;

pub enum RegisterError {
    UsernameTaken,
    AlreadyRegistered,
    InvalidName,
}
pub fn handle_register_command(
    router_state: &mut RouterState,
    client_id: u64,
    name: &String,
) -> Result<(), RegisterError> {
    if name.len() < 3 || name.len() > 24 {
        return Err(RegisterError::InvalidName);
    }

    if router_state.client_names.contains_key(&client_id) {
        return Err(RegisterError::AlreadyRegistered);
    }

    if router_state.client_rev_lookup.contains_key(name) {
        return Err(RegisterError::UsernameTaken);
    }

    router_state
        .client_names
        .insert(client_id, name.to_string());
    router_state
        .client_rev_lookup
        .insert(name.to_string(), client_id);
    return Ok(());
}
