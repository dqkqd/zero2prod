use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_messages::Messages;
use secrecy::SecretString;
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
    skip(state, form, messages),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(state): State<AppState>,
    messages: Messages,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Redirect::to("/").into_response()
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(error) => LoginError::AuthError(error),
                AuthError::UnexpectedError(error) => LoginError::UnexpectedError(error),
            };
            tracing::error!(
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to login"
            );
            messages.error(e.to_string());
            Redirect::to("/login").into_response()
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
