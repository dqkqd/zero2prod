use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
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
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::SubscriberEmail, startup::AppState, telemetry::spawn_blocking_with_tracing};

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
#[tracing::instrument(
    name = "Publish newsletter",
    skip(state, authorization, body),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    State(state): State<AppState>,
    authorization: Option<TypedHeader<Authorization<Basic>>>,
    body: Json<BodyData>,
) -> Result<(), PublishError> {
    let authorization = authorization
        .context("Missing authorization header")
        .map_err(PublishError::AuthError)?;
    tracing::Span::current().record(
        "username",
        tracing::field::display(authorization.username()),
    );
    let user_id = validate_credentials(authorization.0, &state.db_pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

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

#[tracing::instrument(name = "Validate credentials", skip(authorization, pool))]
async fn validate_credentials(
    authorization: Authorization<Basic>,
    pool: &PgPool,
) -> Result<Uuid, PublishError> {
    let (user_id, password_hash) = get_stored_credentials(authorization.username(), pool).await?;

    spawn_blocking_with_tracing(move || {
        verify_password_hash(authorization.password().to_string(), password_hash)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(PublishError::UnexpectedError)??;

    Ok(user_id)
}

#[tracing::instrument(name = "Get stored credentials", skip(pool, username))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<(Uuid, SecretString), PublishError> {
    let row = sqlx::query!(
        r#"
    SELECT user_id, password_hash
    FROM users
    WHERE username = $1;
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")
    .map_err(PublishError::UnexpectedError)?
    .context("Unknown username")
    .map_err(PublishError::AuthError)?;

    Ok((row.user_id, row.password_hash.into()))
}

#[tracing::instrument(name = "Verify password hash", skip(password, expected_password_hash))]
fn verify_password_hash(
    password: String,
    expected_password_hash: SecretString,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.as_bytes(), &expected_password_hash)
        .context("Invalid password")
        .map_err(PublishError::AuthError)?;

    Ok(())
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
