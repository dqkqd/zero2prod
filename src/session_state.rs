use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use tower_sessions::Session;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub async fn renew(&self) -> Result<(), tower_sessions::session::Error> {
        self.0.cycle_id().await
    }

    pub async fn insert_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<(), tower_sessions::session::Error> {
        self.0.insert(Self::USER_ID_KEY, user_id).await
    }

    pub async fn get_user_id(&self) -> Result<Option<Uuid>, tower_sessions::session::Error> {
        self.0.get(Self::USER_ID_KEY).await
    }
}

impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let extractor = Session::from_request_parts(parts, state).await?;
        let extractor = TypedSession(extractor);
        Ok(extractor)
    }
}
