use std::collections::HashMap;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{
    Router,
    body::Body,
    http::{self, HeaderName, HeaderValue, Request, Response},
};
use axum_extra::headers::Authorization;
use axum_extra::headers::Header;
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

#[derive(Clone)]
pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> TestUser {
        TestUser {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub fn auth_header(&self) -> (HeaderName, HeaderValue) {
        let mut header_values = Vec::<HeaderValue>::new();
        let auth = Authorization::basic(&self.username, &self.password);
        auth.encode(&mut header_values);
        let auth_value = header_values.pop().unwrap();
        (http::header::AUTHORIZATION, auth_value)
    }

    async fn store(&self, pool: &PgPool) {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(15000, 2, 1, None).expect("Failed to build Argon2 parameters."),
        );

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(self.password.as_bytes(), &salt)
            .expect("Failed to hash password")
            .to_string();

        sqlx::query!(
            r#"
        INSERT INTO users (user_id, username, password_hash)
        VALUES ($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");
    }
}

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
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

    pub async fn post_newsletters(
        &self,
        body: serde_json::Value,
        basic_auth: Option<&TestUser>,
    ) -> Response<Body> {
        let request = Request::builder()
            .method(http::Method::POST)
            .uri("/newsletters")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());

        let request = match basic_auth {
            Some(user) => {
                let (auth_header, auth_value) = user.auth_header();
                request.header(auth_header, auth_value)
            }
            None => request,
        };

        self.router
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
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

    let app = TestApp {
        router: application.router,
        pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
    };
    app.test_user.store(&app.pool).await;
    app
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
