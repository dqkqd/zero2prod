use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use reqwest::StatusCode;
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::startup::AppState;

pub async fn admin_dashboard(
    State(state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse, DashboardError> {
    let username = if let Some(user_id) = session
        .get::<Uuid>("user_id")
        .await
        .context("cannot get user id from session storage")?
    {
        get_username(user_id, &state.db_pool).await?
    } else {
        todo!()
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
  </body>
</html>
"#
    )))
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, DashboardError> {
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
