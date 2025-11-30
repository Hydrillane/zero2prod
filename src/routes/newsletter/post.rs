use std::fmt;
use actix_web_flash_messages::{FlashMessage, Level};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use actix_web::{http::{header::{HeaderMap, HeaderValue},  StatusCode}, web::{self, Form}, HttpRequest, HttpResponse, ResponseError};
use uuid::Uuid;

use crate::{authentication::{get_username_from_uuid, validate_users_table, AuthError, Credentials}, domain::SubscriberEmail, email_client::EmailClient, idempotency::{save_response, try_processing, IdempotencyKey, NextAction}, middleware::UserID, routes::{e400, e500, error_chain_fmt, see_other}};


#[derive(serde::Deserialize)]
pub struct FormData {
    title:String,
    html_content:String,
    text_content:String,
    idempotency_key:String
}


struct ConfirmedSubscriber {
    email:SubscriberEmail
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Failed Authentication.")]
    AuthenticationError(#[source] anyhow::Error)
}

impl fmt::Debug for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthenticationError(e) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish"#).unwrap();
                response
                    .headers_mut()
                    .insert(actix_web::http::header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}


#[tracing::instrument(
    "Publishing newsletter to confirmed subscriber!",
    skip(form,pool,email_client,user_id),
    fields(username=tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter( 
    form:web::Form<FormData>,
    pool:web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserID>,
) -> Result<HttpResponse,actix_web::Error> {

    let FormData {title,html_content,text_content,idempotency_key} = form.0;
    let idempotency_key:IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let user_id = user_id.into_inner();
    let username = get_username_from_uuid(&pool, *user_id).await.map_err(|e| e500(e.to_string()))?;

    let mut transaction: Transaction<'static, Postgres> = match try_processing(&pool, user_id, &idempotency_key).await.map_err(e500)? {
        NextAction::StartProcessing(t) => {
            t
        }, 
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok(saved_response)
        }
    };

    tracing::Span::current().record(
        "username",
        tracing::field::display(&username)
    );

    let issue_id = insert_newsletter_issue(&mut *transaction, title.as_ref(), text_content.as_ref(), html_content.as_ref())
        .await
        .map_err(e500)?;

    enqueue_newsletter_issue(&mut *transaction, issue_id)
        .await
        .map_err(e500)?;


    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscriber(&pool).await.map_err(e500)?;
    for subscriber in &subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email, 
                    &title, 
                    &html_content,
                    &text_content)
                    .await
                    .map_err(e500)?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                )
            }
        }
    }
    FlashMessage::success(format!("Succesfully sent email to {} subscribers",subscribers.len())).send();
    let response = see_other("/admin/newsletter");
    let response = save_response(transaction, *user_id, response,&idempotency_key)
        .await
        .map_err(e500)?;
    Ok(response)
}

#[tracing::instrument(
    name = "Get all confirmed subscriber",
    skip(pool)
)]
async fn get_confirmed_subscriber(
    pool:&PgPool
) -> Result<Vec<Result<ConfirmedSubscriber,anyhow::Error>>,anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber{email}),
            Err(error) => Err(anyhow::anyhow!(error))
        }
        )
        .collect();
    Ok(row)
}



fn succed_message() -> FlashMessage {
    FlashMessage::warning("The letter has beend published!")
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction:&mut PgConnection,
    title:&str,
    text_content:&str,
    html_content:&str,
) -> Result<Uuid,sqlx::Error>{
    let uuid = Uuid::new_v4();
    let _sqlx = sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issues_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1,$2,$3,$4,now())
        "#,
        uuid,
        title,
        text_content,
        html_content,
    )
        .execute(transaction)
        .await?;
    Ok(uuid)
}

#[tracing::instrument(skip_all)]
async fn enqueue_newsletter_issue(
    transaction: &mut PgConnection,
    newsletter_issue_id:Uuid
) -> Result<(),sqlx::Error> {
    let _sqlx = sqlx::query!(
        r#"
        INSERT INTO issues_delivery_queue (
            newsletter_issues_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id
    )
        .execute(transaction)
        .await?;

    Ok(())
}
