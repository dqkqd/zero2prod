use std::collections::HashMap;

use axum::{
    Router,
    body::Body,
    http::{self, Request, Response},
};
use linkify::{Link, LinkFinder, LinkKind};
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::MockServer;
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
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn get_one(&self, uri: &str, body: Body) -> Response<Body> {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .uri(uri)
                    .body(body)
                    .unwrap(),
            )
            .await
            .expect("failed to execute request")
    }

    pub async fn get_subscriptions_confirm(&self) -> Response<Body> {
        self.get_one("/subscriptions/confirm", Body::empty()).await
    }

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

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: HashMap<String, String> = email_request.body_json().unwrap();

        let get_link = |s: &str| {
            let finder = LinkFinder::new();
            let link: Vec<Link> = finder
                .links(s)
                .filter(|link| link.kind() == &LinkKind::Url)
                .collect();
            assert_eq!(link.len(), 1);
            let link = Url::parse(link[0].as_str()).unwrap();

            //Let's make sure we don't call random API on the Internet
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");

            link
        };

        ConfirmationLinks {
            html: get_link(body["HtmlBody"].as_str()),
            plain_text: get_link(body["TextBody"].as_str()),
        }
    }
}

impl ConfirmationLinks {
    pub fn link_without_host(&self) -> String {
        let mut output = self.html.path().to_string();
        if let Some(query) = self.html.query() {
            output = format!("{output}?{query}");
        }
        if let Some(fragment) = self.html.fragment() {
            output = format!("{output}#{fragment}");
        }
        output
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("failed to get configuration");

        c.database.database_name = Uuid::new_v4().to_string();
        configure_database(&c.database).await;

        c.email_client.base_url = email_server.uri();

        c
    };

    let application = Application::build(configuration.clone());

    TestApp {
        router: application.router,
        pool: get_connection_pool(&configuration.database),
        email_server,
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
