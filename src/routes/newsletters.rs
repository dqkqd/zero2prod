use anyhow::Context;
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Basic},
};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, startup::AppState};

#[derive(Deserialize, Debug)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize, Debug)]
pub struct Content {
    html: String,
    text: String,
}

#[axum::debug_handler]
#[tracing::instrument(name = "Publish newsletter", skip(state))]
pub async fn publish_newsletter(
    State(state): State<AppState>,
    authorization: Option<TypedHeader<Authorization<Basic>>>,
    body: Json<BodyData>,
) -> Result<(), PublishError> {
    let _authorization = authorization
        .context("Missing authorization header")
        .map_err(PublishError::AuthError)?;

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
                        &body.title,
                        &body.content.html,
                        &body.content.text,
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

    Ok(())
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

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl PublishError {
    fn status(&self) -> StatusCode {
        match self {
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response {
        match self {
            PublishError::AuthError(_) => {
                let mut response = self.status().into_response();
                response.headers_mut().insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_static(r#"Basic realm="publish""#),
                );
                response
            }
            _ => (self.status(), self.to_string()).into_response(),
        }
    }
}
