use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect},
};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::{
    authentication::{AuthError, Credentials, validate_credentials},
    startup::AppState,
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[axum::debug_handler]
#[tracing::instrument(
    name = "Login",
    skip(state, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(State(state): State<AppState>, Form(form): Form<FormData>) -> impl IntoResponse {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Redirect::to("/")
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(error) => LoginError::AuthError(error),
                AuthError::UnexpectedError(error) => LoginError::UnexpectedError(error),
            };
            let query_string = format!("error={}", urlencoding::Encoded(e.to_string()));
            let hmac_tag = {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
                    state.hmac_secret.expose_secret().as_bytes(),
                )
                .unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };
            tracing::error!(
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to login"
            );
            Redirect::to(&format!("/login?{query_string}&tag={hmac_tag:x}"))
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}
