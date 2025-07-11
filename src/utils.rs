use axum::response::IntoResponse;
use axum_messages::Messages;
use reqwest::StatusCode;

pub struct E500(anyhow::Error);

impl IntoResponse for E500 {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

impl<E> From<E> for E500
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        E500(err.into())
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
