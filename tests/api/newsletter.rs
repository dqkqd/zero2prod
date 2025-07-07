use axum::http::StatusCode;
use rstest::rstest;
use uuid::Uuid;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::helpers::{ConfirmationLinks, TestApp, TestUser, spawn_app};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_newsletters(serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .await;

    assert_eq!(response.status().as_u16(), StatusCode::OK);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_newsletters(serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .await;

    assert_eq!(response.status().as_u16(), StatusCode::OK);
}

#[rstest]
#[case::missing_title(serde_json::json!({
    "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    }
}))]
#[case::missing_content(serde_json::json!({
    "title": "Newsletter!"
}))]
#[tokio::test]
async fn newsletter_return_422_for_invalid_data(#[case] invalid_body: serde_json::Value) {
    let app = spawn_app().await;
    let response = app.post_newsletters(invalid_body).await;
    assert_eq!(response.status().as_u16(), StatusCode::UNPROCESSABLE_ENTITY,);
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
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

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let response = app
        .client
        .post(format!("{}/newsletters", app.address()))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("failede to execute request");

    assert_eq!(response.status().as_u16(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;

    let test_user = TestUser::generate();
    let response = app
        .client
        .post(format!("{}/newsletters", app.address()))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .basic_auth(test_user.username, Some(test_user.password))
        .send()
        .await
        .expect("failede to execute request");

    assert_eq!(response.status().as_u16(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let app = spawn_app().await;
    let mut user = app.test_user.clone();
    user.password = Uuid::new_v4().to_string();
    assert_ne!(app.test_user.password, user.password);

    let response = app
        .client
        .post(format!("{}/newsletters", app.address()))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .basic_auth(user.username, Some(user.password))
        .send()
        .await
        .expect("failede to execute request");

    assert_eq!(response.status().as_u16(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}
