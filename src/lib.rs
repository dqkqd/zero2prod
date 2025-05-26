use axum::{Router, routing::get};

pub fn app() -> Router {
    Router::new().route("/health_check", get(health_check))
}
async fn health_check() {}

pub async fn run(listener: tokio::net::TcpListener) -> std::io::Result<()> {
    let app = app();
    axum::serve(listener, app).await?;
    Ok(())
}
