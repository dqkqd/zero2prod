use axum::response::{Html, IntoResponse};

pub async fn home() -> impl IntoResponse {
    Html(include_str!("home.html"))
}
