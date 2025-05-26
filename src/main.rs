use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health_check", get(health_check));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() {}
