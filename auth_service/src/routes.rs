use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};

use crate::handlers::{AppState, login, register, root, verify};

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(root))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/verify", post(verify))
        .layer(cors)
        .with_state(state)
}
