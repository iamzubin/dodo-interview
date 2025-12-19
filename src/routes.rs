use axum::{
    middleware::{self},
    routing::{get, post},
    Router,
};
use crate::handlers::{auth, health, accounts};
use crate::state::AppState; 
use crate::middlewares::auth::auth_middleware;

pub fn create_router(state: AppState) -> Router<AppState> {

    // Auth routes
    let auth_routes = Router::new()
        .route("/generate-api-key", post(auth::generate_api_key))
        .route("/signup", post(auth::signup));

    // Accounts routes
    let accounts_routes = Router::new()
        .route("/create", post(accounts::create_account))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));



    Router::new()
        .route("/", get(health::health_check))
        .nest("/accounts", accounts_routes)
        .nest("/auth", auth_routes)
}

