use axum::response::{Html, IntoResponse};
use axum_messages::Messages;

use crate::utils::get_all_messages;

#[axum::debug_handler]
pub async fn login_form(messages: Messages) -> impl IntoResponse {
    let message = get_all_messages(messages);
    Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Login</title>
  </head>
  <body>
    {message}
    <form action="/login" method="post">
      <label
        >Username
        <input type="text" placeholder="Enter Username" name="username" />
      </label>
      <label
        >Password
        <input type="password" placeholder="Enter Password" name="password" />
      </label>

      <button type="submit">Login</button>
    </form>
  </body>
</html>
            "#
    ))
}
