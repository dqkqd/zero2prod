use rstest::rstest;
use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    assert_ne!(&new_password, &app.test_user.password);
    let response = app
        .post_change_password(serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;

    app.post_login(serde_json::json!({
            "username": app.test_user.username,
            "password": app.test_user.password,
    }))
    .await;

    let new_password = Uuid::new_v4().to_string();
    assert_ne!(&new_password, &app.test_user.password);
    let response = app
        .post_change_password(serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &new_password,
            "new_password_check": new_password + "123",
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
         the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;

    app.post_login(serde_json::json!({
            "username": app.test_user.username,
            "password": app.test_user.password,
    }))
    .await;

    let new_password = Uuid::new_v4().to_string();
    assert_ne!(&new_password, &app.test_user.password);
    let response = app
        .post_change_password(serde_json::json!({
            "current_password": app.test_user.password.to_string() + "123",
            "new_password": &new_password,
            "new_password_check": new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[rstest]
#[case::password_must_be_at_least_12_characters(
    "ab1xc2s3d40",
    "New password must be at least 12 characters."
)]
#[case::password_must_be_less_than_128_characters(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "New password must be less than 128 characters."
)]
#[tokio::test]
async fn password_security(#[case] password: &'static str, #[case] error: &'static str) {
    let app = spawn_app().await;

    app.post_login(serde_json::json!({
            "username": app.test_user.username,
            "password": app.test_user.password,
    }))
    .await;

    let response = app
        .post_change_password(serde_json::json!({
            "current_password": app.test_user.password.to_string(),
            "new_password": password,
            "new_password_check": password,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(&format!("<p><i>{error}</i></p>")));
}

#[tokio::test]
async fn changing_password_works() {
    let app = spawn_app().await;

    let response = app
        .post_login(serde_json::json!({
                "username": app.test_user.username,
                "password": app.test_user.password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();
    assert_ne!(&new_password, &app.test_user.password);
    let response = app
        .post_change_password(serde_json::json!({
            "current_password": app.test_user.password.to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let response = app
        .post_login(serde_json::json!({
                "username": app.test_user.username,
                "password": new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}
