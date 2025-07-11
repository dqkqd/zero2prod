use axum::response::Redirect;
use axum_messages::Messages;

use crate::{session_state::TypedSession, utils::E500};

#[axum::debug_handler]
pub async fn logout(message: Messages, session: TypedSession) -> Result<Redirect, E500> {
    if session.get_user_id().await?.is_some() {
        session.log_out().await?;
        message.info("You have successfully logged out.");
    }
    Ok(Redirect::to("/login"))
}
