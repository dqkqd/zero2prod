use axum::Json;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize, Debug)]
pub struct Content {
    html: String,
    text: String,
}

#[axum::debug_handler]
#[tracing::instrument(name = "Publish newsletter")]
pub async fn publish_newsletter(payload: Json<Content>) {}
