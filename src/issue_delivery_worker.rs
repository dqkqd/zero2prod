use std::time::Duration;

use anyhow::Context;
use sqlx::PgPool;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool, utils::Transaction,
};

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let sender_email = configuration
        .email_client
        .sender()
        .map_err(|_| anyhow::anyhow!("invalid sender email address"))?;

    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    worker_loop(connection_pool, email_client).await;

    Ok(())
}

#[tracing::instrument(name = "Run worker loop", skip(pool, email_client))]
async fn worker_loop(pool: PgPool, email_client: EmailClient) {
    loop {
        match try_execute_task(&email_client, &pool).await {
            Ok(ExecutionOutCome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Ok(ExecutionOutCome::TaskCompleted) => {}
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

#[derive(Debug)]
struct NewsletterIssue {
    newsletter_issue_id: uuid::Uuid,
    title: String,
    text_content: String,
    html_content: String,
}

#[derive(Debug)]
pub enum ExecutionOutCome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(name = "Try execute task", skip(pool, email_client), err(Debug))]
pub async fn try_execute_task(
    email_client: &EmailClient,
    pool: &PgPool,
) -> Result<ExecutionOutCome, anyhow::Error> {
    let txn = pool.begin().await.context("cannot open transaction")?;
    if let Some((txn, newsletter_issue_id, subscriber_email)) = dequeue_task(txn).await? {
        let subscriber_email = match SubscriberEmail::parse(subscriber_email) {
            Ok(email) => email,
            Err(err) => {
                tracing::error!(
                    error.cause_chain = ?err,
                    "Skipping a confirmed subscriber.\
                    Their stored contact details are invalid",
                );
                anyhow::bail!(err)
            }
        };

        let task = get_newsletter_issue(pool, newsletter_issue_id).await?;
        if let Err(err) = email_client
            .send_email(
                &subscriber_email,
                &task.title,
                &task.html_content,
                &task.text_content,
            )
            .await
        {
            tracing::error!(
                error.cause_chain = ?err,
                "Failed to delivery issue to a confirmed subscriber. Skipping."
            )
        }

        delete_task(txn, task).await?;
        Ok(ExecutionOutCome::TaskCompleted)
    } else {
        Ok(ExecutionOutCome::EmptyQueue)
    }
}

#[tracing::instrument(name = "Dequeue task", skip(txn), err(Debug), ret)]
async fn dequeue_task(
    mut txn: Transaction,
) -> Result<Option<(Transaction, uuid::Uuid, String)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
SELECT newsletter_issue_id, subscriber_email
FROM issue_delivery_queue
FOR UPDATE
SKIP LOCKED
LIMIT 1
        "#
    )
    .fetch_optional(&mut *txn)
    .await?;

    match row {
        Some(row) => Ok(Some((txn, row.newsletter_issue_id, row.subscriber_email))),
        None => Ok(None),
    }
}

#[tracing::instrument(name = "Delete task", skip(txn), err(Debug))]
async fn delete_task(mut txn: Transaction, task: NewsletterIssue) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
DELETE FROM issue_delivery_queue
WHERE newsletter_issue_id = $1
        "#,
        task.newsletter_issue_id
    )
    .execute(&mut *txn)
    .await?;
    txn.commit().await?;
    Ok(())
}

#[tracing::instrument(name = "Get newsletter issue", skip(pool), err(Debug), ret)]
async fn get_newsletter_issue(
    pool: &PgPool,
    newsletter_issue_id: uuid::Uuid,
) -> Result<NewsletterIssue, anyhow::Error> {
    let row = sqlx::query!(
        r#"
SELECT title, text_content, html_content
FROM newsletter_issues
WHERE newsletter_issue_id = $1
                "#,
        newsletter_issue_id
    )
    .fetch_one(pool)
    .await?;

    Ok(NewsletterIssue {
        newsletter_issue_id,
        title: row.title,
        text_content: row.text_content,
        html_content: row.html_content,
    })
}
