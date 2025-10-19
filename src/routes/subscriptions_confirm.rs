use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;


#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(parameters)
)]
pub async fn confirm(
    parameters:web::Query<Parameters>,
    pool: web::Data<PgPool>,
    ) -> HttpResponse {

    let id = match get_subscriber_id_from_token(&pool,parameters.0.subscription_token).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::Unauthorized().finish(),
    };

    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => {
            if subscriber_confirm(subscriber_id, &pool).await.is_err() {
                HttpResponse::InternalServerError().finish()
            } else {
                HttpResponse::Ok().finish()
            }
        }
    }
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
     .await
     .map_err(|e| {
         println!("Failed to update subscriber as confirmed!");
             e
     })?;
    Ok(())
}

#[tracing::instrument(
    name = "Get / Select Subscriber Id from Token",
    skip(subscription_token)
)]
async fn get_subscriber_id_from_token(
    pool:&PgPool,
    subscription_token:String,
) -> Result<Option<Uuid>,sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_tokens = $1"#,
        subscription_token,
    )
      .fetch_optional(pool)
      .await
      .map_err(|e| {
          println!("Failed to execute query {}",e);
          e
      })?;    
    Ok(result.map(|r| r.subscriber_id))
}


