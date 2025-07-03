use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    http::Request,
    routing::{get, post},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::trace::TraceLayer;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{confirm, health_check, publish_newsletter, subscribe},
};

pub struct Application {
    pub address: String,
    pub router: Router,
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: Arc<EmailClient>,
    pub base_url: String,
}

impl Application {
    pub fn build(configuration: Settings) -> Application {
        let connection_pool = get_connection_pool(&configuration.database);
        let sender_email = configuration
            .email_client
            .sender()
            .expect("invalid sender email address");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = configuration.application.address();

        let state = AppState {
            db_pool: connection_pool,
            email_client: Arc::new(email_client),
            base_url: configuration.application.base_url,
        };
        let router = router(state);

        Application { address, router }
    }

    pub async fn run_until_stopped(self) -> std::io::Result<()> {
        let listener = tokio::net::TcpListener::bind(self.address).await?;
        tracing::info!("listening on {}", listener.local_addr().unwrap());
        axum::serve(listener, self.router).await?;
        Ok(())
    }
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let span = tracing::debug_span!(
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

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}
