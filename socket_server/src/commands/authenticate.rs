use jsonwebtoken::{TokenData, Validation, decode};

use crate::{protocol::Claims, state::RouterState};

pub enum AuthenticateError {
    InvalidToken,
}

pub fn handle_authenticate_command(
    router_state: &mut RouterState,
    client_id: u64,
    token: &str,
) -> Result<String, AuthenticateError> {
    let claims: TokenData<Claims> =
        match decode::<Claims>(token, &router_state.decoding_key, &Validation::default()) {
            Err(e) => {
                eprintln!("error validating token: {e}");
                return Err(AuthenticateError::InvalidToken);
            }
            Ok(c) => c,
        };

    router_state
        .connection_to_user
        .insert(client_id, claims.claims.user.id);
    Ok(claims.claims.user.id.to_string())
}
