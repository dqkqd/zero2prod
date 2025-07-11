use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::{
    authentication::{self, Credentials, validate_credentials},
    routes::admin::dashboard::get_username,
    session_state::TypedSession,
    startup::AppState,
    utils::E500,
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
) -> Result<Redirect, E500> {
    let user_id = match session.get_user_id().await? {
        Some(user_id) => user_id,
        None => return Ok(Redirect::to("/login")),
    };

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Ok(change_password_redirect(
            "You entered two different new passwords - the field values must match.",
            messages,
        ));
    }

    let username = get_username(user_id, &state.db_pool).await?;
    let credentials = Credentials {
        username,
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

    authentication::change_password(user_id, form.new_password, &state.db_pool).await?;
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
