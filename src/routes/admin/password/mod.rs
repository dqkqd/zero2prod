mod get;
mod post;

use axum::response::IntoResponse;
pub use get::change_password_form;
pub use post::change_password;
use reqwest::StatusCode;

#[derive(thiserror::Error, Debug)]
pub enum ChangePasswordError {
    #[error("{0}")]
    PasswordError(String),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl ChangePasswordError {
    fn status(&self) -> StatusCode {
        match self {
            ChangePasswordError::PasswordError(_) => StatusCode::BAD_REQUEST,
            ChangePasswordError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ChangePasswordError {
    fn into_response(self) -> axum::response::Response {
        (self.status(), self.to_string()).into_response()
    }
}
