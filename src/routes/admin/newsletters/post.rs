use anyhow::Context;
use axum::{
    Extension, Form,
    body::Body,
    extract::State,
    http::Response,
    response::{IntoResponse, Redirect},
};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::PgConnection;

use crate::{
    authentication::CurrentUser,
    idempotency::{IdempotencyKey, NextAction, save_response, try_processing},
    startup::AppState,
    utils::{AppError, e400},
};

#[derive(Deserialize, Debug)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

#[axum::debug_handler]
pub async fn publish_newsletters(
    State(state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response<Body>, AppError> {
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let txn = state
        .db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool.")?;

    let mut txn = match try_processing(txn, &idempotency_key, current_user.user_id).await? {
        NextAction::StartProcessing(txn) => txn,
        NextAction::ReturnSavedResponse(response) => {
            messages.info("Successfully published a newsletter.");
            return Ok(response);
        }
    };

    let newsletter_issue_id =
        insert_newsletter_issue(&mut txn, &title, &text_content, &html_content)
            .await
            .context("cannot insert newsletter_issue")?;
    enqueue_delivery_tasks(&mut txn, newsletter_issue_id)
        .await
        .context("failed to enqueue delivery tasks")?;

    messages.info("Successfully published a newsletter.");
    let response = Redirect::to("/admin/newsletters").into_response();

    let response = save_response(txn, &idempotency_key, current_user.user_id, response).await?;

    Ok(response)
}

async fn insert_newsletter_issue(
    txn: &mut PgConnection,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<uuid::Uuid, sqlx::Error> {
    let newsletter_issue_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"
INSERT INTO newsletter_issues (
    newsletter_issue_id,
    title,
    text_content,
    html_content,
    published_at
)
VALUES ($1, $2, $3, $4, now())
        "#,
        &newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(txn)
    .await?;
    Ok(newsletter_issue_id)
}

async fn enqueue_delivery_tasks(
    txn: &mut PgConnection,
    newsletter_issue_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
INSERT INTO issue_delivery_queue  (
    newsletter_issue_id,
    subscriber_email
)
SELECT $1, email
FROM subscriptions WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(txn)
    .await?;
    Ok(())
}
