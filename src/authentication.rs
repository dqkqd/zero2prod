use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    let (user_id, password_hash) = match get_stored_credentials(&credentials.username, pool).await?
    {
        Some((user_id, password_hash)) => (Some(user_id), password_hash),
        None => (
            None,
            "$argon2id$v=19$m=15000,t=2,p=1$\
                gySEVMmPuRVG7WfKGI3kkA$\
                jLtcZMQ/KWaNuM2q7nYYcGan0wijjF7hCAYa56V28Ts"
                .into(),
        ),
    };

    spawn_blocking_with_tracing(move || verify_password_hash(credentials.password, password_hash))
        .await
        .context("Failed to spawn blocking task.")
        .map_err(AuthError::UnexpectedError)??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username"))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(pool, username))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretString)>, AuthError> {
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
    .map_err(AuthError::UnexpectedError)?;

    Ok(row.map(|r| (r.user_id, r.password_hash.into())))
}

#[tracing::instrument(name = "Verify password hash", skip(password, expected_password_hash))]
fn verify_password_hash(
    password: SecretString,
    expected_password_hash: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)?;

    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
