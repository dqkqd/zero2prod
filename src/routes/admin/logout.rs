use axum::response::Redirect;
use axum_messages::Messages;

use crate::{
    session_state::TypedSession,
    utils::{AppError, e500},
};

#[axum::debug_handler]
pub async fn logout(message: Messages, session: TypedSession) -> Result<Redirect, AppError> {
    if session.get_user_id().await.map_err(e500)?.is_some() {
        session.log_out().await.map_err(e500)?;
        message.info("You have successfully logged out.");
    }
    Ok(Redirect::to("/login"))
}
