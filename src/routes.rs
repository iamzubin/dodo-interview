use axum::{
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use crate::handlers::{auth, user};

pub fn create_router() -> Router {

    // Auth routes
    let auth_routes = Router::new()
        .route("/login", post(auth::login))
        .route("/signup", post(auth::signup))
        .route("/logout", post(auth::logout));

    // User routes
    let user_routes = Router::new()
        .route("/", post(user::create_user).get(user::list_users))
        .route(
            "/{id}",
            get(user::get_user)
                .put(user::update_user)
                .delete(user::delete_user),
        )
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .route("/", get(health_check))
        .nest("/users", user_routes)
        .nest("/auth", auth_routes)
}

async fn auth_middleware(request: Request, next: Next) -> Response {
    // Placeholder for authentication logic
    println!("Auth middleware triggered for: {}", request.uri());
    next.run(request).await
}

async fn health_check() -> &'static str {
    "Server is running!"
}