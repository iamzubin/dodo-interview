use crate::handlers::{accounts, auth, health};
use crate::middlewares::auth::{auth_middleware, ApiKeyExtractor};
use crate::state::AppState;
use axum::{
    middleware::{self},
    routing::{get, post},
    Router,
};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::{Any, CorsLayer};

pub fn create_router(state: AppState) -> Router<AppState> {
    // Rate limiting configuration
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .key_extractor(ApiKeyExtractor)
        .finish()
        .unwrap();

    let governor_layer = GovernorLayer::new(governor_conf);

    // Auth routes
    let auth_routes = Router::new()
        .route("/generate-api-key", post(auth::generate_api_key))
        .route("/signup", post(auth::signup));

    // Protected accounts routes
    let protected_accounts_routes = Router::new()
        .route("/create", post(accounts::create_account))
        .route("/transfer", post(accounts::transfer))
        .route("/credit-debit", post(accounts::credit_debit))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(governor_layer.clone());

    // Protected webhooks routes
    let protected_webhooks_routes = Router::new()
        .route(
            "/register",
            post(crate::handlers::webhooks::register_webhook_handler),
        )
        .route(
            "/list",
            get(crate::handlers::webhooks::list_webhooks_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(governor_layer);

    // Public accounts routes
    let public_accounts_routes = Router::new().route("/", get(accounts::get_accounts));

    Router::new()
        .route("/", get(health::health_check))
        .nest(
            "/accounts",
            public_accounts_routes.merge(protected_accounts_routes),
        )
        .nest("/auth", auth_routes)
        .nest("/webhooks", protected_webhooks_routes)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
