
use actix_web::{web, HttpResponse};
use chrono::{Offset, Utc};
use sqlx::{query, types::time::OffsetDateTime, PgPool};
use tracing::{info_span, instrument, Instrument};
use uuid::Uuid;

use crate::FormData;



#[tracing::instrument(
    name="Starting subscriber function got triggered",
    skip(form,pool),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email
    )
)]
pub async fn subscribe(
    form:web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> HttpResponse {

    match query_to_subscriptions(&form,&pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }

}

#[tracing::instrument (
    name = " Saving query to subscriptions ",
    skip(form,pool)
)]
async fn query_to_subscriptions(
    form: &FormData,
    pool: &PgPool,
) -> Result<(),sqlx::Error> {

    sqlx::query!(
        r#"
        INSERT INTO subscriptions ( id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to save/ query to database: {:?}",e);
        e
    })?;
    Ok(())

}
