use anyhow::Context;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    session_state::TypedSession,
    startup::AppState,
    utils::{AppError, e500},
};

#[derive(Clone)]
pub struct CurrentUser {
    pub user_id: Uuid,
    pub username: String,
}

pub async fn reject_anonymous_users(
    State(state): State<AppState>,
    session: TypedSession,
    mut request: Request,
    next: Next,
) -> Result<impl IntoResponse, AppError> {
    match session
        .get_user_id()
        .await
        .context("cannot get user id from session storage")
        .map_err(e500)?
    {
        Some(user_id) => {
            let username = get_username(&user_id, &state.db_pool).await?;
            request
                .extensions_mut()
                .insert(CurrentUser { user_id, username });
            let response = next.run(request).await;
            Ok(response.into_response())
        }
        None => Ok(Redirect::to("/login").into_response()),
    }
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: &Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
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
