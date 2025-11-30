use std::{clone, time::Duration};

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use tracing::{field::{self, display}, Span, Subscriber};
use uuid::Uuid;

use crate::{configuration::{self, Setting}, domain::SubscriberEmail, email_client::EmailClient, startup::get_connection_pool};

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}


#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = field::Empty,
        subscriber_email = field::Empty,
    ),
    err
)]
pub async fn try_execute_task(
    pool:&PgPool,
    email_client:&EmailClient
) -> Result<ExecutionOutcome,sqlx::Error> {
    let task = dequeue_task(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue)
    }; 
    if let Some((transaction,issue_id,email)) = dequeue_task(pool).await? {
        Span::current()
            .record("newsletter_issue_id", &display(&issue_id))
            .record("subscriber_email", &display(&email));
        match SubscriberEmail::parse(email.clone()) {
            Ok(t) => {
                let issue = select_from_newsletter_issues_id(pool, issue_id).await?;
                if let Err(e) = email_client.send_email(
                    &t, 
                    &issue.title, 
                    &issue.html_content, 
                    &issue.text_content
                )
                    .await {
                        tracing::error!(
                            error.cause_chain = %e,
                            error.message = %e,
                            "Failed to deliver email to subscriber ! \
                            Skipping.",
                        )

                }
            }
            Err(e) => {
                tracing::error!(
                    error.cause_chain = %e,
                    error.message = %e,
                    "Skipping a confirmed subscriber. \
                    their stored contact are invalid"
                )
            }
        }
        delete_task(transaction,issue_id,email).await?;
    
    }
    Ok(ExecutionOutcome::TaskCompleted)
}

pub async fn dequeue_task(
    pool:&PgPool
) -> Result<Option<(PgTransaction,Uuid,String)>,sqlx::Error> {
    let transaction = pool.begin().await?;
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issues_id, subscriber_email
        FROM issues_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#,
    )
        .fetch_optional(pool)
        .await?;

    if let Some(r) = r {
        Ok(Some((
                    transaction,
                    r.newsletter_issues_id,
                    r.subscriber_email
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(
    skip_all,
)]
pub async fn delete_task(
    mut transaction:PgTransaction,
    newsletter_issues_id:Uuid,
    subscriber_email:String
) -> Result<(),sqlx::Error> {

    let _r = sqlx::query!(
        r#"
        DELETE FROM issues_delivery_queue
        WHERE
        newsletter_issues_id = $1 AND
        subscriber_email = $2
        "#,
        newsletter_issues_id,
        subscriber_email
    )
        .execute(&mut *transaction)
        .await?;
    Ok(())
}

pub struct NewsletterIssue {
    title:String,
    text_content:String,
    html_content:String,
}

#[tracing::instrument(skip_all)]
pub async fn select_from_newsletter_issues_id(
    pool:&PgPool,
    newsletter_issues_id:Uuid
) -> Result<NewsletterIssue,sqlx::Error> {
    let query = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE newsletter_issues_id = $1
        "#,
        newsletter_issues_id
    ).fetch_one(pool)
        .await?;
    Ok(query)
}

#[tracing::instrument(skip_all)]
pub async fn workers_loop(
    pool:&PgPool,
    email_client:&EmailClient
) -> Result<(),anyhow::Error> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            },
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }

}

pub async fn run_worker_until_stopped(
    setting:Setting
) -> Result<(),anyhow::Error> {
    let connection_pool = get_connection_pool(&setting.database);
    let sender = setting.email_client.sender().expect("Failed to get email sender");
    let timeout = setting.email_client.timeout();
    // let email_client = EmailClient::new(
    //     setting.application.base_url,
    //     sender,
    //     setting.email_client.authorization_token,
    //     timeout);
    let email_client = setting.email_client.client();
    workers_loop(&connection_pool, &email_client).await

} 

type PgTransaction = Transaction<'static,Postgres>;

