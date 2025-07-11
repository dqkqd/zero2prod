use anyhow::Context;
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
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

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: Uuid,
    password: SecretString,
    pool: &PgPool,
) -> Result<(), AuthError> {
    let password_hash = compute_password_hash(password).map_err(AuthError::UnexpectedError)?;
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $2
        WHERE user_id = $1
            "#,
        user_id,
        password_hash.expose_secret(),
    )
    .execute(pool)
    .await
    .context("Failed to update  user")
    .map_err(AuthError::UnexpectedError)?;

    Ok(())
}

pub fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(15000, 2, 1, None).context("Failed to build Argon2 parameters.")?,
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)
    .context("Failed to hash password")?
    .to_string();

    Ok(password_hash.into())
}
