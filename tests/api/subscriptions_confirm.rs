use reqwest::StatusCode;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = app
        .client
        .get(format!("{}/subscriptions/confirm", app.address()))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST)
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = app
        .client
        .get(confirmation_links.html)
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK)
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    app.client
        .get(confirmation_links.html)
        .send()
        .await
        .expect("failed to execute request");

    let saved = sqlx::query!("SELECT email, name, status from subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscriptions");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
