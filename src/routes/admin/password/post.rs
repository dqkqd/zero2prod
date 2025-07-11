use anyhow::Context;
use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::{
    authentication::{Credentials, validate_credentials},
    routes::{ChangePasswordError, admin::dashboard::get_username},
    session_state::TypedSession,
    startup::AppState,
};

#[derive(Deserialize, Debug)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

#[axum::debug_handler]
pub async fn change_password(
    State(state): State<AppState>,
    messages: Messages,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<Redirect, ChangePasswordError> {
    let user_id = match session
        .get_user_id()
        .await
        .context("cannot get user id from session storage")
        .map_err(ChangePasswordError::UnexpectedError)?
    {
        Some(user_id) => user_id,
        None => return Ok(Redirect::to("/login")),
    };

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Ok(change_password_redirect(
            ChangePasswordError::PasswordError(
                "You entered two different new passwords - the field values must match.".into(),
            ),
            messages,
        ));
    }

    let username = get_username(user_id, &state.db_pool)
        .await
        .context("cannot get password for user id")
        .map_err(ChangePasswordError::UnexpectedError)?;

    let credentials = Credentials {
        username,
        password: form.current_password,
    };
    if validate_credentials(credentials, &state.db_pool)
        .await
        .is_err()
    {
        return Ok(change_password_redirect(
            ChangePasswordError::PasswordError("The current password is incorrect.".into()),
            messages,
        ));
    }

    if let Err(e) = validate_password_security(form.new_password) {
        return Ok(change_password_redirect(e, messages));
    }
    todo!()
}

fn change_password_redirect(e: ChangePasswordError, messages: Messages) -> Redirect {
    tracing::error!(
        error.message = %e,
        error.cause_chain = ?e,
        "Failed to change password"
    );
    messages.error(e.to_string());
    Redirect::to("/admin/password")
}

fn validate_password_security(password: SecretString) -> Result<(), ChangePasswordError> {
    if password.expose_secret().len() < 12 {
        return Err(ChangePasswordError::PasswordError(
            "New password must be at least 12 characters.".into(),
        ));
    }
    if password.expose_secret().len() >= 128 {
        return Err(ChangePasswordError::PasswordError(
            "New password must be less than 128 characters.".into(),
        ));
    }
    Ok(())
}
