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
use sqlx::PgPool;

use crate::{
    authentication::CurrentUser,
    domain::SubscriberEmail,
    idempotency::{IdempotencyKey, get_saved_response, save_response},
    startup::AppState,
    utils::{AppError, e400, e500},
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
    let subscribers = get_confirmed_subscribers(&state.db_pool)
        .await
        .context("failed to get confirmed subscribers.")?;
    let idempotency_key: IdempotencyKey = form.idempotency_key.try_into().map_err(e400)?;
    if let Some(response) =
        get_saved_response(&state.db_pool, &idempotency_key, current_user.user_id)
            .await
            .map_err(e500)?
    {
        messages.info("Successfully published a newsletter.");
        return Ok(response);
    }

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(
                        &subscriber.email,
                        &form.title,
                        &form.html_content,
                        &form.text_content,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "failed to send newsletter issue to {}",
                            subscriber.email.as_ref()
                        )
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber.\
                    Their stored contact details are invalid",

                )
            }
        }
    }

    messages.info("Successfully published a newsletter.");
    let response = Redirect::to("/admin/newsletters").into_response();

    let response = save_response(
        &state.db_pool,
        &idempotency_key,
        current_user.user_id,
        response,
    )
    .await
    .map_err(e500)?;

    Ok(response)
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email FROM subscriptions WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}
