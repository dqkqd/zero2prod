use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    Router,
    body::Body,
    http::Request,
    routing::{get, post},
};
use axum_messages::MessagesManagerLayer;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_sessions::{SessionManagerLayer, cookie::Key};
use tower_sessions_redis_store::{
    RedisStore,
    fred::{self, prelude::ClientLike},
};

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{
        admin_dashboard, confirm, health_check, home, login, login_form, publish_newsletter,
        subscribe,
    },
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
    pub hmac_secret: SecretString,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Application, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let sender_email = configuration
            .email_client
            .sender()
            .map_err(|_| anyhow::anyhow!("invalid sender email address"))?;

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
            hmac_secret: configuration.application.hmac_secret,
        };

        let redis_pool = get_redis_connection_pool(&configuration.redis_uri)
            .context("failed to create redis pool")?;
        redis_pool
            .init()
            .await
            .context("failed to connect to redis")?;

        let router = router(state, redis_pool);

        Ok(Application { address, router })
    }

    pub async fn run_until_stopped(self, listener: TcpListener) -> std::io::Result<()> {
        tracing::info!("listening on {}", listener.local_addr().unwrap());
        axum::serve(listener, self.router).await?;
        Ok(())
    }
}

fn router(state: AppState, redis_pool: fred::prelude::Pool) -> Router {
    let session_store = RedisStore::new(redis_pool);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_signed(Key::from(state.hmac_secret.expose_secret().as_bytes()));

    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .route("/", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/admin/dashboard", get(admin_dashboard))
        .with_state(state)
        .layer(MessagesManagerLayer)
        .layer(session_layer)
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

pub fn get_redis_connection_pool(
    redis_uri: &SecretString,
) -> Result<fred::prelude::Pool, anyhow::Error> {
    // https://github.com/aembke/fred.rs/blob/main/examples/axum.rs
    let config = fred::prelude::Config::from_url(redis_uri.expose_secret())?;
    let pool = fred::prelude::Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
        .set_policy(fred::prelude::ReconnectPolicy::new_exponential(
            0, 100, 30_000, 2,
        ))
        .build_pool(8)?;
    Ok(pool)
}
