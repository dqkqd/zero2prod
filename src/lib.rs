use axum::{Router, routing::get};

pub async fn run() -> std::io::Result<()> {
    let app = Router::new().route("/health_check", get(health_check));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() {}
