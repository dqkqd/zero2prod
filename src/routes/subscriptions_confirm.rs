use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::startup::AppState;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[axum::debug_handler]
#[tracing::instrument(name = "Confirm a pending subscriber", skip(state, parameters), err)]
pub async fn confirm(
    State(state): State<AppState>,
    parameters: Query<Parameters>,
) -> Result<(), StatusCode> {
    let subscriber_id =
        get_subscriber_id_from_token(&state.db_pool, &parameters.subscription_token)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let subscriber_id = subscriber_id.ok_or(StatusCode::UNAUTHORIZED)?;

    confirm_subscriber(&state.db_pool, subscriber_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool), err)]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to execute query");
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(subscription_token, pool),
    err
)]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let record = sqlx::query!(
        r#"
        SELECT subscriber_id
        FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to execute query");
        e
    })?;

    Ok(record.map(|r| r.subscriber_id))
}
