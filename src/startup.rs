use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::Request,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{
    email_client::EmailClient,
    routes::{health_check, subscribe},
};

pub fn app(pool: PgPool, email_client: EmailClient) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool)
        .with_state(Arc::new(email_client))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let span = tracing::info_span!(
                    "request",
                    method=?request.method(),
                    uri=?request.uri(),
                    version=?request.version(),
                    request_id = tracing::field::Empty
                );
                if let Some(id) = span.id() {
                    span.record("request_id", id.into_u64());
                }
                span
            }),
        )
}

pub async fn run(
    listener: tokio::net::TcpListener,
    pool: PgPool,
    email_client: EmailClient,
) -> std::io::Result<()> {
    let app = app(pool, email_client);
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;
    Ok(())
}
