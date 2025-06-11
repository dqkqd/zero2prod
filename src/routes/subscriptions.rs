use axum::{Form, extract::State, http::StatusCode};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(Debug, Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[axum::debug_handler]
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    ),
    )
]
pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<(), StatusCode> {
    let new_subscriber = NewSubscriber {
        email: form.email,
        name: SubscriberName::parse(form.name).expect("name validation failed."),
    };
    insert_subscriber(&pool, &new_subscriber)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email,
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to execute query");
        e
    })?;
    Ok(())
}
