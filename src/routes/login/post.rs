use axum::{
    Form,
    extract::State,
    response::Redirect,
};
use axum_messages::Messages;
use secrecy::SecretString;
use serde::Deserialize;
use tower_sessions::Session;

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
    skip(state, form, messages, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(state): State<AppState>,
    messages: Messages,
    session: Session,
    Form(form): Form<FormData>,
) -> Redirect {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));

            if let Err(e) = session.cycle_id().await {
                return login_redirect(LoginError::UnexpectedError(e.into()), messages);
            }
            if let Err(e) = session.insert("user_id", user_id).await {
                return login_redirect(LoginError::UnexpectedError(e.into()), messages);
            }
            Redirect::to("/admin/dashboard")
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(error) => LoginError::AuthError(error),
                AuthError::UnexpectedError(error) => LoginError::UnexpectedError(error),
            };
            login_redirect(e, messages)
        }
    }
}

fn login_redirect(e: LoginError, messages: Messages) -> Redirect {
    tracing::error!(
        error.message = %e,
        error.cause_chain = ?e,
        "Failed to login"
    );
    messages.error(e.to_string());
    Redirect::to("/login")
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}
