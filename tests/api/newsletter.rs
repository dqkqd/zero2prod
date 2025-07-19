use axum::http::StatusCode;
use rstest::rstest;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::helpers::{ConfirmationLinks, TestApp, assert_is_redirect_to, spawn_app};

fn when_sending_an_email() -> wiremock::MockBuilder {
    Mock::given(matchers::path("/email")).and(matchers::method("POST"))
}

#[tokio::test]
async fn you_must_be_logged_in_to_send_newsletter() {
    let app = spawn_app().await;
    let response = app.get_newsletters().await;
    assert_is_redirect_to(&response, "/login");

    let response = app.login_test_user().await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let response = app.get_newsletters().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.login_test_user().await;
    create_unconfirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_newsletters(serde_json::json!({
            "title": "Newsletter title",
            "html_content": "<p>Newsletter body as HTML</p>",
            "text_content": "Newsletter body as plain text",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>Successfully published a newsletter.</i></p>"));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    app.login_test_user().await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_newsletters(serde_json::json!({
            "title": "Newsletter title",
            "html_content": "<p>Newsletter body as HTML</p>",
            "text_content": "Newsletter body as plain text",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>Successfully published a newsletter.</i></p>"));
}

#[rstest]
#[case::missing_title(serde_json::json!({
    "html_content": "<p>Newsletter body as HTML</p>",
    "text_content": "Newsletter body as plain text",
}))]
#[case::missing_content(serde_json::json!({
    "title": "Newsletter!"
}))]
#[tokio::test]
async fn newsletter_return_422_for_invalid_data(#[case] invalid_body: serde_json::Value) {
    let app = spawn_app().await;
    app.login_test_user().await;
    let response = app.post_newsletters(invalid_body).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    app.login_test_user().await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });

    let response = app.post_newsletters(newsletter_request_body.clone()).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>Successfully published a newsletter.</i></p>"));

    let response = app.post_newsletters(newsletter_request_body.clone()).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>Successfully published a newsletter.</i></p>"));
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;
    app.login_test_user().await;
    create_confirmed_subscriber(&app).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });

    let response1 = app.post_newsletters(newsletter_request_body.clone());
    let response2 = app.post_newsletters(newsletter_request_body.clone());
    let (response1, response2) = tokio::join!(response1, response2);

    assert_is_redirect_to(&response1, "/admin/newsletters");
    assert_is_redirect_to(&response2, "/admin/newsletters");
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let _mock_guard = when_sending_an_email()
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .named("create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions("name=le%20guin&email=ursula_le_guin%40gmail.com")
        .await;

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    app.client
        .get(confirmation_links.html)
        .send()
        .await
        .expect("failed to execute request");
}
