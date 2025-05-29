use axum::{
    Router,
    routing::{get, post},
};

use crate::routes::{health_check, subscribe};

pub fn app() -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
}

pub async fn run(listener: tokio::net::TcpListener) -> std::io::Result<()> {
    let app = app();
    axum::serve(listener, app).await?;
    Ok(())
}
