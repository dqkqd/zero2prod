use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_LENGTH},
};
use tower::ServiceExt;

use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let response = app
        .router
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
