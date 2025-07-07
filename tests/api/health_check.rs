use axum::http::{StatusCode, header::CONTENT_LENGTH};

use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let response = app
        .client
        .get(format!("{}/health_check", app.address()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(CONTENT_LENGTH).unwrap(), &"0");
}
