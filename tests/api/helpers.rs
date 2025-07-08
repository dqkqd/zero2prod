use std::collections::HashMap;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::http::{self};
use linkify::{Link, LinkFinder, LinkKind};
use once_cell::sync::Lazy;
use reqwest::{StatusCode, Url};
use sqlx::{PgPool, postgres::PgPoolOptions};
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
    pub client: reqwest::Client,
    pub port: u16,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub fn address(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub async fn post_subscriptions(&self, body: &'static str) -> reqwest::Response {
        self.client
            .post(format!("{}/subscriptions", self.address()))
            .header(
                http::header::CONTENT_TYPE,
                mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
            )
            .body(body)
            .send()
            .await
            .expect("failed to execute request")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.client
            .post(format!("{}/newsletters", self.address()))
            .json(&body)
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .send()
            .await
            .expect("failed to execute request")
    }

    pub async fn post_login(&self, body: serde_json::Value) -> reqwest::Response {
        self.client
            .post(format!("{}/login", self.address()))
            .form(&body)
            .send()
            .await
            .expect("failed to execute request")
    }

    pub async fn get_login_html(&self) -> String {
        self.client
            .get(format!("{}/login", self.address()))
            .send()
            .await
            .expect("failed to execute request")
            .text()
            .await
            .unwrap()
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
            let mut link = Url::parse(link[0].as_str()).unwrap();

            //Let's make sure we don't call random API on the Internet
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");

            link.set_port(Some(self.port)).unwrap();

            link
        };

        ConfirmationLinks {
            html: get_link(body["HtmlBody"].as_str()),
            plain_text: get_link(body["TextBody"].as_str()),
        }
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let configuration = {
        let mut c = get_configuration().expect("failed to get configuration");

        c.database.database_name = Uuid::new_v4().to_string();
        configure_database(&c.database).await;

        c.email_client.base_url = email_server.uri();

        c.application.port = address.port();

        c
    };

    let application = Application::build(configuration.clone())
        .await
        .expect("failed to build application");

    tokio::spawn(async move {
        application
            .run_until_stopped(listener)
            .await
            .expect("sever error")
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let app = TestApp {
        client,
        port: address.port(),
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

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get(http::header::LOCATION).unwrap(),
        location
    );
}

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
