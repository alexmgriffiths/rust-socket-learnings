use crate::state::RouterState;

pub fn handle_disconnected_event(router_state: &mut RouterState, client_id: u64) {
    router_state.disconnect_client(client_id);
}
