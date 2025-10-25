
use std::fmt;

use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use serde::Deserialize;
use sqlx::PgPool;
use tracing_subscriber::fmt::Formatter;
use uuid::Uuid;
use anyhow::{Error,Context};


#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String
}

#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error(transparent)]
    UnexpectedError(#[from] Error),
    #[error("There is no subscriber token associated with the provided token!")]
    UnknownToken,
}


impl ResponseError for ConfirmError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ConfirmError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmError::UnknownToken => StatusCode::UNAUTHORIZED,
        }

    }
}


impl fmt::Debug for ConfirmError {
    fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn error_chain_fmt (
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

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(parameters)
)]
pub async fn confirm(
    parameters:web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse,ConfirmError> {

    let id = get_subscriber_id_from_token(&pool,&parameters.subscription_token)
        .await
        .context("Failed to get subscriber id from the database token!")?
        .ok_or(ConfirmError::UnknownToken)?;

    subscriber_confirm(id, &pool)
        .await
        .context("Failed to confirm subscriber id from the token in database!")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Mark subscriber as confirmed"
)]
async fn subscriber_confirm(
    subscriber_id: Uuid,
    pool:&PgPool
) -> Result<(),sqlx::Error> {
    let _result = sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
        .execute(pool)
        .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Get / Select Subscriber Id from Token",
    skip(subscription_token)
)]
async fn get_subscriber_id_from_token(
    pool:&PgPool,
    subscription_token:&str,
) -> Result<Option<Uuid>,sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_tokens = $1"#,
        subscription_token,
    )
        .fetch_optional(pool)
        .await
      ?;    
    Ok(result.map(|r| r.subscriber_id))
}


