use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::startup::AppState;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[axum::debug_handler]
#[tracing::instrument(name = "Adding a new subscriber", skip(_state, _parameters), err)]
pub async fn confirm(
    State(_state): State<AppState>,
    _parameters: Query<Parameters>,
) -> Result<(), StatusCode> {
    Ok(())
}
