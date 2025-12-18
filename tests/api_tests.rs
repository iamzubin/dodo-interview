use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt; // for collecting body
use tower::ServiceExt; // for one_shot

#[tokio::test]
async fn health_check() {
    let app = dodointerview::create_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"Server is running!");
}
