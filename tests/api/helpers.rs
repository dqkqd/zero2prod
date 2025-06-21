use axum::{
    Router,
    body::Body,
    http::{self, Request, Response},
};
use once_cell::sync::Lazy;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;
use zero2prod::{
    configuration::{DatabaseSettings, get_configuration},
    startup::{Application, get_connection_pool},
    telemetry::init_subscriber,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber("debug".into());
    }
});

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: &'static str) -> Response<Body> {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/subscriptions")
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .expect("failed to execute request")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let mut configuration = get_configuration().expect("failed to get configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone());

    TestApp {
        router: application.router,
        pool: get_connection_pool(&configuration.database),
    }
}

async fn configure_database(configuration: &DatabaseSettings) {
    let connection_pool = PgPoolOptions::new()
        .connect_with(configuration.without_db())
        .await
        .expect("can't connect to database");
    sqlx::query(&format!(
        r#"CREATE DATABASE "{}""#,
        &configuration.database_name
    ))
    .execute(&connection_pool)
    .await
    .expect("can't create database");

    let connection_pool = get_connection_pool(configuration);
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("can't migrate database");
}
