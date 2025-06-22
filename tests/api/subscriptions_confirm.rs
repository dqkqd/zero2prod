use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;
    let response = app.get_subscriptions_confirm().await;
    assert_eq!(response.status().as_u16(), 400)
}
