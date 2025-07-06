use axum::{
    extract::Query,
    response::{Html, IntoResponse},
};
use htmlescape::encode_minimal;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct QueryParams {
    error: Option<String>,
}

pub async fn login_form(Query(query): Query<QueryParams>) -> impl IntoResponse {
    let error_html = match query.error {
        Some(error) => format!("<p><i>{}</i></p>", encode_minimal(&error)),
        None => "".to_string(),
    };
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
