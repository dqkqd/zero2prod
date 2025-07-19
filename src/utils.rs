use axum::response::IntoResponse;
use axum_messages::Messages;
use reqwest::StatusCode;

pub fn e500<E>(e: E) -> AppError
where
    E: Into<anyhow::Error>,
{
    AppError::UnexpectedError(e.into())
}

pub fn e400<E>(e: E) -> AppError
where
    E: Into<anyhow::Error>,
{
    AppError::BadRequest(e.into())
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status(), self.to_string()).into_response()
    }
}

pub fn get_all_messages(messages: Messages) -> String {
    let mut messages = messages.into_iter().collect::<Vec<_>>();
    messages.sort_by_key(|m| std::cmp::Reverse(m.level));
    messages
        .iter()
        .map(|m| format!("<p><i>{}</i></p>", m.message))
        .collect::<Vec<_>>()
        .join("")
}
