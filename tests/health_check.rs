use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_LENGTH},
};
use tower::ServiceExt;
use zero2prod::app;

#[tokio::test]
async fn health_check_works() {
    let app = app();

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
