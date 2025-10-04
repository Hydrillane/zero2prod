
use actix_web::{web, HttpResponse};
use sqlx::{types::time::OffsetDateTime, PgPool};
use uuid::Uuid;

use crate::{domain::NewSubscriber, email_client:: EmailClient, FormData};



#[tracing::instrument(
    name="Starting subscriber function got triggered",
    skip(form,pool,email_client),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email,
    )
)]
pub async fn subscribe(
    form:web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {

    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish()
    };
    if query_to_subscriptions(&new_subscriber, &pool).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };

    if send_confirmation_email(&email_client, new_subscriber)
        .await
            .is_err()
    {
        return HttpResponse::InternalServerError().finish()
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument (
    name = " Saving query to subscriptions ",
    skip(new_subscriber,pool)
)]
async fn query_to_subscriptions(
    new_subscriber: &NewSubscriber,
    pool: &PgPool,
) -> Result<(),sqlx::Error> {

    sqlx::query!(
        r#"
        INSERT INTO subscriptions ( id, email, name, subscribed_at,status)
        VALUES ($1, $2, $3, $4, 'pending_confirmations')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        OffsetDateTime::now_utc(),
    )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save/ query to database: {:?}",e);
            e
        })?;
    Ok(())

}

#[tracing::instrument (
    name = "Send a confirmation email to a new subscriber",
    skip(email_client,new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber
) -> Result<(),reqwest::Error> {
    let confirmation_link = 
        "https://my-api.com/subscriptions/confirm";

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
        new_subscriber.email, 
        "Welcome!", 
        &html_body,
        &plain_body,
    )
        .await
}
