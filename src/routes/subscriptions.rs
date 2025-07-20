use anyhow::Context;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::AppState,
    utils::Transaction,
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::parse(form.email)?;
        let name = SubscriberName::parse(form.name)?;
        Ok(NewSubscriber { email, name })
    }
}

#[axum::debug_handler]
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, state),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    ),
    err(Debug),
    )
]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<(), SubscribeError> {
    let new_subscriber: NewSubscriber =
        form.try_into().map_err(SubscribeError::ValidiationError)?;
    let mut txn = state
        .db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool.")?;

    let subscriber_id = insert_subscriber(&mut txn, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;

    let subscription_token = generate_subscription_token();
    store_token(txn, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;

    send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email")?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, txn)
)]
pub async fn insert_subscriber(
    txn: &mut PgConnection,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(&mut *txn)
    .await?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, txn)
)]
pub async fn store_token(
    mut txn: Transaction,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    )
    .execute(&mut *txn)
    .await?;

    txn.commit().await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(new_subscriber, email_client)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={subscription_token}");
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription.",
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await?;

    Ok(())
}

fn generate_subscription_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 25)
}

#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidiationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl SubscribeError {
    fn status(&self) -> StatusCode {
        match self {
            SubscribeError::ValidiationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> Response {
        (self.status(), self.to_string()).into_response()
    }
}
