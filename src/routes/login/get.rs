use axum::response::{Html, IntoResponse};
use axum_extra::extract::CookieJar;

#[axum::debug_handler]
pub async fn login_form(jar: CookieJar) -> impl IntoResponse {
    let error_html = match jar.get("_flash") {
        Some(cookie) => format!("<p><i>{}</i></p>", cookie.value()),
        None => "".into(),
    };
    (
        jar.remove("_flash"),
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
        )),
    )
}
