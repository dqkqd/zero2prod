use axum::{
    Form, Router,
    routing::{get, post},
};
use serde::Deserialize;

pub async fn run(listener: tokio::net::TcpListener) -> std::io::Result<()> {
    let app = app();
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn app() -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
}

async fn health_check() {}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FormData {
    name: String,
    email: String,
}
async fn subscribe(Form(user): Form<FormData>) {
    dbg!(user);
}
