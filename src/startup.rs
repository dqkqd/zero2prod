use axum::{
    Router,
    body::Body,
    http::Request,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::routes::{health_check, subscribe};

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let request_id = Uuid::new_v4();
                tracing::debug_span!(
                "request",
                method=?request.method(),
                uri=?request.uri(),
                version=?request.version(),
                request_id=tracing::field::display(request_id)
                )
            }),
        )
}

pub async fn run(listener: tokio::net::TcpListener, pool: PgPool) -> std::io::Result<()> {
    let app = app(pool);
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;
    Ok(())
}
