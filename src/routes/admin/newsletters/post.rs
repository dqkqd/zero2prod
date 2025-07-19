use anyhow::Context;
use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail,
    startup::AppState,
};

#[derive(Deserialize, Debug)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
}

#[axum::debug_handler]
pub async fn publish_newsletters(
    State(state): State<AppState>,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Redirect, AppError> {
    let subscribers = get_confirmed_subscribers(&state.db_pool)
        .await
        .context("failed to get confirmed subscribers.")?;
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
    Ok(Redirect::to("/admin/newsletters"))
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
