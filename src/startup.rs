use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

use crate::routes::{health_check, subscribe};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool)
}

pub async fn run(listener: tokio::net::TcpListener, pool: PgPool) -> std::io::Result<()> {
    let app = app(pool);
    axum::serve(listener, app).await?;
    Ok(())
}
