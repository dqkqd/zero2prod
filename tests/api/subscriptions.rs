use std::collections::HashMap;

use axum::http::StatusCode;
use linkify::{Link, LinkFinder, LinkKind};
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

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: HashMap<String, String> = email_request.body_json().unwrap();

    let get_link = |s: &str| {
        let finder = LinkFinder::new();
        let link: Vec<Link> = finder
            .links(s)
            .filter(|link| link.kind() == &LinkKind::Url)
            .collect();
        assert_eq!(link.len(), 1);
        link[0].as_str().to_string()
    };

    let html_link = get_link(body["HtmlBody"].as_str());
    let text_link = get_link(body["HtmlBody"].as_str());
    assert_eq!(html_link, text_link);
}

#[tokio::test]
async fn subscriber_persists_the_new_subscriber() {
    let app = spawn_app().await;
    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;

    let saved = sqlx::query!("SELECT email, name, status from subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscriptions");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}
