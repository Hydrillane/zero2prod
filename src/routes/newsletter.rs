use std::fmt;

use anyhow::Context;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{self, subscriber};
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};

use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};

#[derive(Deserialize,Debug)]
pub struct BodyData {
    title:String,
    content:Content,
}

#[derive(Deserialize,Debug)]
struct Content {
    html:String,
    text:String
}

struct ConfirmedSubscriber {
    email:SubscriberEmail
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

impl fmt::Debug for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}



#[tracing::instrument(
    "Publishing newsletter to confirmed subscriber!"
    ,skip(body,pool,email_client)
)]
pub async fn publish_newsletter(
    body:web::Json<BodyData>,
    pool:web::Data<PgPool>,
    email_client: web::Data<EmailClient>) -> Result<HttpResponse,PublishError> {

    let subscribers = get_confirmed_subscriber(&pool).await?;
    for subscriber in subscribers {
        email_client.send_email(
            subscriber.email, 
            &body.title, 
            &body.content.html, 
            &body.content.text)
            .await
            .with_context(|| {
                "Failed to send email to the subscriber?"
            })?;
    }
}

#[tracing::instrument(
    name = "Get all confirmed subscriber",
    skip(pool)
)]
async fn get_confirmed_subscriber(
    pool:&PgPool
) -> Result<Vec<ConfirmedSubscriber>,anyhow::Error> {
    struct Row {
        email: String
    }
    let row = sqlx::query_as!(
        Row,
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
        .fetch_all(pool)
        .await?;
    let confirmed_subscriber = 
        row
        .into_iter()
        .filter_map(|r| {
            match SubscriberEmail::parse(r.email) {
                Ok(email) => Some(ConfirmedSubscriber { email }) ,
                Err(error) => {
                    tracing::warn!(
                        "Failed to get confirmed subscriber for {}",
                        error
                    );
                    None
                }
            }
        })
    .collect();
    Ok(confirmed_subscriber)
}
