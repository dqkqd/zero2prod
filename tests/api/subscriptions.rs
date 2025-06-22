use axum::http::StatusCode;
use rstest::rstest;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_return_a_200_for_valid_form_data() {
    let app = spawn_app().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app
        .post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let saved = sqlx::query!("SELECT name, email from subscriptions")
        .fetch_one(&app.pool)
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
async fn subscribe_return_a_422_when_data_is_missing(#[case] invalid_body: &'static str) {
    let app = spawn_app().await;
    let response = app.post_subscriptions(invalid_body).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[rstest]
#[case::empty_name("name=&email=ursula_le_guin%40gmail.com")]
#[case::empty_email("name=Ursula&email=")]
#[case::invalid_email("name=Ursula&email=definitely-not-an-email")]
#[tokio::test]
async fn subscribe_return_a_422_when_fields_are_present_but_invalid(
    #[case] invalid_body: &'static str,
) {
    let app = spawn_app().await;
    let response = app.post_subscriptions(invalid_body).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;
}
