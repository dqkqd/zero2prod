use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::startup::AppState;

#[derive(Deserialize, Debug)]
pub struct QueryParams {
    error: Option<String>,
    tag: Option<String>,
}

impl QueryParams {
    fn verify(self, secret: SecretString) -> Result<String, anyhow::Error> {
        match (self.error, self.tag) {
            (Some(error), Some(tag)) => {
                let tag = hex::decode(tag)?;
                let query_string = format!("error={}", urlencoding::Encoded(&error));

                let mut mac =
                    Hmac::<sha2::Sha256>::new_from_slice(secret.expose_secret().as_bytes())
                        .unwrap();
                mac.update(query_string.as_bytes());
                mac.verify_slice(&tag)?;
                Ok(error)
            }
            (None, _) => Ok("".to_string()),
            (Some(_), None) => anyhow::bail!("Missing tag in query parameter"),
        }
    }
}

#[axum::debug_handler]
pub async fn login_form(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> impl IntoResponse {
    let error_html = match query.verify(state.hmac_secret) {
        Ok(error) => format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error)),
        Err(e) => {
            tracing::warn!(
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to verify query parameters using the HMAC tag"
            );
            "".to_string()
        }
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
