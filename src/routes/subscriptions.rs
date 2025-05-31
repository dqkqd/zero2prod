use axum::{Form, extract::State};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FormData {
    name: String,
    email: String,
}

#[axum::debug_handler]
pub async fn subscribe(State(pool): State<PgPool>, Form(user): Form<FormData>) {
    dbg!(user, pool);
}
