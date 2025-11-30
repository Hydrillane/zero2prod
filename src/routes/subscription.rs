
use std::fmt;

use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use sqlx::{types::time::OffsetDateTime, PgConnection, PgPool};
use uuid::Uuid;
use rand::{distr::Alphanumeric, rng, Rng};
use anyhow::{Error,Context};

use crate::{domain::NewSubscriber, email_client:: EmailClient, startup::ApplicationBaseUrl, FormData};



#[tracing::instrument(
    name="Starting subscriber function got triggered",
    skip(form,pool,email_client,base_url),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email,
    )
)]
pub async fn subscribe(
    form:web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse,SubscriberError> {

    let mut transaction = pool.begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let new_subscriber = form.0.try_into().map_err(|e| SubscriberError::ValidationError(e))?;

    let subscriber_id = query_to_subscriptions(&new_subscriber, &mut *transaction)
        .await
        .context("Failed to insert new subscriber in the database")?;

    let subscription_token = generate_subscriptions_token();

    store_token(&mut *transaction, subscriber_id, subscription_token.as_ref())
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;

    transaction.commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber")?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token)
        .await
        .context("Failed to send a confirmation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument (
    name = " Saving query to subscriptions ",
    skip(new_subscriber,transaction)
)]
async fn query_to_subscriptions(
    new_subscriber: &NewSubscriber,
    transaction: &mut PgConnection
) -> Result<Uuid,sqlx::Error> {

    let subs_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO subscriptions ( id, email, name, subscribed_at,status)
        VALUES ($1, $2, $3, $4, 'pending_confirmations')
        "#,
        subs_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        OffsetDateTime::now_utc(),
    )
        .execute(transaction)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save/ query to database: {:?}",e);
            e
        })?;

    Ok(subs_id)

}

#[tracing::instrument (
    name = "Send a confirmation email to a new subscriber",
    skip(email_client,new_subscriber,base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url:&str,
    subscription_token: &str
) -> Result<(),reqwest::Error> {
    let confirmation_link =
        format!("{}/subscriptions/confirm?subscription_token={}",
            base_url,
            subscription_token);

    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscriptions",
        confirmation_link
    );

    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscriptions.",
        confirmation_link
    );


    email_client.send_email(
        &new_subscriber.email, 
        "Welcome!", 
        &html_body,
        &plain_body,
    )
        .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_tokens,transaction)
)]
pub async fn store_token(
    transaction:&mut PgConnection,
    subscriber_id:Uuid,
    subscription_tokens: &str
) -> Result<(),StoreTokenError> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_tokens, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_tokens,
        subscriber_id
    )
        .execute(transaction)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}",e);
            StoreTokenError(e)
        })?;

    Ok(())

}


fn generate_subscriptions_token() -> String {

    let mut rng = rng();

    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()

}


#[derive(Debug,thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] Error),
  }

// impl std::error::Error for SubscriberError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             SubscriberError::ValidationError(_) => None,
//             SubscriberError::StoreTokenError(e) => Some(e),
//             SubscriberError::PoolError(e) => Some(e),
//             SubscriberError::InsertSubscriberError(e) => Some(e),
//             SubscriberError::ExecuteError(e) => Some(e),
//             SubscriberError::SendEmailError(e) => Some(e)
//
//
//         }
//     }
// }

// impl std::fmt::Display for SubscriberError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             SubscriberError::ValidationError(e) => write!(f,"{}",e),
//             SubscriberError::StoreTokenError(_) => write!(f," Failed to store the confirmation token for new subscriber"),
//             SubscriberError::PoolError(_) => write!(f,"Failed to pool to the database !"),
//             SubscriberError::InsertSubscriberError(_) => write!(f,"Failed to insert new subscriber to the database!"),
//             SubscriberError::ExecuteError(_) => write!(f,"Failed to execute the query to the database!"),
//             SubscriberError::SendEmailError(_) => write!(f,"Failed to send the subscriptions token to the subscriber email!"),
//
//         }
//     }
// }


impl ResponseError for SubscriberError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            SubscriberError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscriberError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                   }
    }
}

pub struct StoreTokenError(sqlx::Error);

impl std::error::Error for StoreTokenError {}

impl fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A database error was encountered while 
            trying to store a subscriptions token"
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}


pub fn error_chain_fmt (
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>
) -> std::fmt::Result {
    writeln!(f,"{}\n",e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f,"Caused by:\n\t{}",cause)?;
        current = cause.source();
    }
    Ok(())
}
//
// impl From<String> for SubscriberError {
//     fn from(e: String) -> Self {
//         SubscriberError::ValidationError(e)
//     }
// }
//
// impl From<StoreTokenError> for SubscriberError {
//     fn from(e: StoreTokenError) -> Self {
//         SubscriberError::StoreTokenError(e)
//     }
// }
// //
// impl From<reqwest::Error> for SubscriberError {
//     fn from(e: reqwest::Error) -> Self {
//         SubscriberError::SendEmailError(e)
//     }
// }
