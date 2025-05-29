use axum::Form;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FormData {
    name: String,
    email: String,
}
pub async fn subscribe(Form(user): Form<FormData>) {
    dbg!(user);
}
