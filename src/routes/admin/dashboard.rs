use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{session_state::TypedSession, startup::AppState};

#[axum::debug_handler]
pub async fn admin_dashboard(
    State(state): State<AppState>,
    session: TypedSession,
) -> Result<impl IntoResponse, DashboardError> {
    let username = if let Some(user_id) = session
        .get_user_id()
        .await
        .context("cannot get user id from session storage")?
    {
        get_username(user_id, &state.db_pool).await?
    } else {
        return Ok(Redirect::to("/login").into_response());
    };

    Ok(Html(format!(
        r#"
<!doctype html>
<html lang="en">
  <head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <title>Admin dashboard</title>
  </head>
  <body>
    <p> Welcome {username}!</p>
    <p>Available actions:</p>
    <ol>
      <li><a href="/admin/password">Change password</a></li>
    </ol>
  </body>
</html>
"#
    ))
    .into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, DashboardError> {
    let row = sqlx::query!(
        r#"
    SELECT username
    FROM users
    WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username")?;
    Ok(row.username)
}

#[derive(thiserror::Error, Debug)]
pub enum DashboardError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl DashboardError {
    fn status(&self) -> StatusCode {
        match self {
            DashboardError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for DashboardError {
    fn into_response(self) -> axum::response::Response {
        (self.status(), self.to_string()).into_response()
    }
}
