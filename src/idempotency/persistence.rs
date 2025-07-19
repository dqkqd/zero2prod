use std::str::FromStr;

use anyhow::Context;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::idempotency::IdempotencyKey;

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response<Body>>, anyhow::Error> {
    // https://docs.rs/sqlx/latest/sqlx/macro.query.html#force-a-differentcustom-type
    match sqlx::query!(
        r#"
    SELECT
        response_status_code,
        response_headers AS "response_headers!: Vec<HeaderPairRecord>",
        response_body
    FROM idempotency
    WHERE
        user_id = $1 AND idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
    )
    .fetch_optional(pool)
    .await?
    {
        Some(r) => {
            let status = r
                .response_status_code
                .try_into()
                .context("negative status code")
                .map(StatusCode::from_u16)
                .context("invalid status code")??;
            let mut headers = HeaderMap::new();
            for header in r.response_headers {
                let name = HeaderName::from_str(&header.name);
                let value = HeaderValue::from_bytes(&header.value);
                match (name, value) {
                    (Ok(name), Ok(value)) => {
                        headers.insert(name, value);
                    }
                    _ => tracing::error!("invalid name and value"),
                };
            }

            let response = (status, headers, r.response_body).into_response();
            Ok(Some(response))
        }
        None => Ok(None),
    }
}

pub async fn save_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: Response,
) -> Result<Response, anyhow::Error> {
    let status = response.status();
    let headers = response.headers().clone();
    let header_records: Vec<HeaderPairRecord> = response
        .headers()
        .iter()
        .map(|(name, value)| HeaderPairRecord {
            name: name.to_string(),
            value: value.as_bytes().to_vec(),
        })
        .collect();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .context("Cannot convert body stream to bytes")?;

    sqlx::query_unchecked!(
        r#"
INSERT INTO idempotency
(
    user_id,
    idempotency_key,
    response_status_code,
    response_headers,
    response_body,
    created_at
)
VALUES ($1, $2, $3, $4, $5, now())
        "#,
        user_id,
        idempotency_key.as_ref(),
        status.as_u16() as i16,
        &header_records,
        body.as_ref(),
    )
    .execute(pool)
    .await?;

    let response = (status, headers, body).into_response();
    Ok(response)
}
