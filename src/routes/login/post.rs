use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
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
    skip(state, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, LoginError> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &state.db_pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(error) => LoginError::AuthError(error),
            AuthError::UnexpectedError(error) => LoginError::UnexpectedError(error),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    Ok(Redirect::to("/"))
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        let encoded_error = urlencoding::Encoded(self.to_string());
        Redirect::to(&format!("/login?error={encoded_error}")).into_response()
    }
}
