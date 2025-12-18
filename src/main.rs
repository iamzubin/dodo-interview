use std::net::SocketAddr;

use dodointerview::create_router;

#[tokio::main]
async fn main() {
    let app = create_router();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}