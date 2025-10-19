
use actix_web::{web, HttpResponse};
use sqlx::{types::time::OffsetDateTime, PgConnection, PgPool};
use uuid::Uuid;
use rand::{distr::Alphanumeric, rng, Rng};

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
) -> HttpResponse {

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish()
    };

    let subscriber_id = match query_to_subscriptions(&new_subscriber, &mut *transaction).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscription_token = generate_subscriptions_token();
    if store_token(&mut *transaction, subscriber_id, subscription_token.as_ref())
        .await
        .is_err() 
    {
        return HttpResponse::InternalServerError().finish()
    }

    if transaction.commit()
        .await
        .is_err() {
            return HttpResponse::InternalServerError().finish()
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        "mytoken")
        .await
            .is_err()
    {
        return HttpResponse::InternalServerError().finish()
    }
    HttpResponse::Ok().finish()
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
        new_subscriber.email, 
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
) -> Result<(),sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_tokens, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_tokens,
        subscriber_id
    )
        .execute(transaction)
        .await
        .map_err(|e| {
            tracing::error!("Failed to store token to table!");
            e
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
