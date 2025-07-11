use anyhow::Context;
use axum::response::{Html, IntoResponse, Redirect};
use axum_messages::{Level, Messages};

use crate::{routes::ChangePasswordError, session_state::TypedSession};

#[axum::debug_handler]
pub async fn change_password_form(
    messages: Messages,
    session: TypedSession,
) -> Result<impl IntoResponse, ChangePasswordError> {
    if session
        .get_user_id()
        .await
        .context("cannot get user id from session storage")
        .map_err(ChangePasswordError::UnexpectedError)?
        .is_none()
    {
        return Ok(Redirect::to("/login").into_response());
    };

    let error_html = messages
        .into_iter()
        .filter(|m| m.level == Level::Error)
        .map(|m| format!("<p><i>{}</i></p>", m.message))
        .collect::<Vec<_>>()
        .join("");

    Ok(Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Change password</title>
  </head>
  <body>
    {error_html}
    <form action="/admin/password" method="post">
      <label
        >Current password
        <input type="password" placeholder="Enter current password" name="current_password" />
      </label>
      <br>
      <label
        >New password
        <input type="password" placeholder="Enter new password" name="new_password" />
      </label>
      <br>
      <label
        >Confirm new password
        <input type="password" placeholder="Type the new password again" name="new_password_check" />
      </label>
      <br>
      <button type="submit">Login</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
  </body>
</html>
            "#,
    )).into_response())
}
