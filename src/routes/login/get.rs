use axum::response::{Html, IntoResponse};
use axum_messages::{Level, Messages};

#[axum::debug_handler]
pub async fn login_form(messages: Messages) -> impl IntoResponse {
    let error_html = messages
        .into_iter()
        .filter(|m| m.level == Level::Error)
        .map(|m| format!("<p><i>{}</i></p>", m.message))
        .collect::<Vec<_>>()
        .join("");

    Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Login</title>
  </head>
  <body>
    {error_html}
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
