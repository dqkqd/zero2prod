use axum::response::Html;
use axum_messages::Messages;

use crate::utils::get_all_messages;

#[axum::debug_handler]
pub async fn change_password_form(messages: Messages) -> Html<String> {
    let message = get_all_messages(messages);
    Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Change password</title>
  </head>
  <body>
    {message}
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
    ))
}
