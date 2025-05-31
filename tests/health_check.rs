use axum::{
    Router,
    body::Body,
    http::{self, Request, StatusCode, header::CONTENT_LENGTH},
};
use rstest::rstest;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use zero2prod::{app, configuration::get_configuration};

async fn spawn_app() -> Router {
    let settings = get_configuration().expect("failed to get configuration");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings.database.connection_string())
        .await
        .expect("can't connect to database");
    app(pool)
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health_check")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(CONTENT_LENGTH).unwrap(), &"0");
}

#[tokio::test]
async fn subscribe_return_a_200_for_valid_form_data() {
    let app = spawn_app().await;

    let settings = get_configuration().expect("failed to get configuration");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings.database.connection_string())
        .await
        .expect("can't connect to database");

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/subscriptions")
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .body(Body::from(
                    "name=le%20guin&email=ursula_le_guin%40gmail.com",
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let saved = sqlx::query!("SELECT name, email from subscriptions")
        .fetch_one(&pool)
        .await
        .expect("failed to fetch saved subscriptions");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[rstest]
#[case::missing_the_email("name=le%20guin")]
#[case::missing_the_name("email=ursula_le_guin%40gmail.com")]
#[case::missing_both_name_and_email("")]
#[tokio::test]
async fn subscribe_return_a_400_when_data_is_missing(#[case] invalid_body: &'static str) {
    let app = spawn_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/subscriptions")
                .header(
                    http::header::CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                )
                .body(Body::from(invalid_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
