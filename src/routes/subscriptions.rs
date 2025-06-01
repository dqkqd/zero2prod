use axum::{Form, extract::State, http::StatusCode};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[axum::debug_handler]
pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<(), StatusCode> {
    tracing::info!(
        "adding '{}' '{}' as a new subscriber",
        form.email,
        form.name
    );

    tracing::info!("saving new subscriber detail in the database");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(&pool)
    .await
    {
        Ok(_) => {
            tracing::info!("new subscriber details have been saved");
            Ok(())
        }
        Err(e) => {
            tracing::error!("failed to execute query: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
