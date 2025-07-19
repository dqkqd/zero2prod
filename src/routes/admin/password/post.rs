use axum::{Extension, Form, extract::State, response::Redirect};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::{
    authentication::{self, Credentials, CurrentUser, validate_credentials},
    startup::AppState,
    utils::{AppError, e500},
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
    Extension(current_user): Extension<CurrentUser>,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Redirect, AppError> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Ok(change_password_redirect(
            "You entered two different new passwords - the field values must match.",
            messages,
        ));
    }

    let username = current_user.username;
    let credentials = Credentials {
        username: username.clone(),
        password: form.current_password,
    };
    if validate_credentials(credentials, &state.db_pool)
        .await
        .is_err()
    {
        return Ok(change_password_redirect(
            "The current password is incorrect.",
            messages,
        ));
    }

    if let Err(e) = validate_password_security(&form.new_password) {
        return Ok(change_password_redirect(e, messages));
    }

    authentication::change_password(&username, form.new_password, &state.db_pool)
        .await
        .map_err(e500)?;
    messages.info("Your password has been changed.");
    Ok(Redirect::to("/admin/password"))
}

fn change_password_redirect(e: &'static str, messages: Messages) -> Redirect {
    tracing::error!(error.message = %e, "Failed to change password");
    messages.error(e);
    Redirect::to("/admin/password")
}

fn validate_password_security(password: &SecretString) -> Result<(), &'static str> {
    if password.expose_secret().len() < 12 {
        return Err("New password must be at least 12 characters.");
    }
    if password.expose_secret().len() >= 128 {
        return Err("New password must be less than 128 characters.");
    }
    Ok(())
}
