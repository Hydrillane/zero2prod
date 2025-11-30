use actix_web::{body::to_bytes, http::{self, StatusCode}, HttpResponse};
use sqlx::{prelude::Type, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{idempotency::IdempotencyKey, middleware::UserID};

#[derive(Debug,Type)]
#[sqlx(type_name="header_pair")]
struct HeaderPairRecord {
    name:String,
    value:Vec<u8>
}

pub enum NextAction {
    StartProcessing(Transaction<'static,Postgres>),
    ReturnSavedResponse(HttpResponse)
}

pub async fn try_processing(
    pool:&PgPool,
    user_id:UserID,
    idempotency_key:&IdempotencyKey
) -> Result<NextAction,anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let n_inserted = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1,$2,now())
        ON CONFLICT DO NOTHING
        "#,
        *user_id,
        idempotency_key.as_ref(),
    )
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    if n_inserted > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved = get_saved_response(&pool, *user_id, idempotency_key)
            .await?
            .ok_or_else(||
                anyhow::anyhow!("Expected saved response, found nothing!"))?;
        Ok(NextAction::ReturnSavedResponse(saved))
    }


}

// impl PgHasArrayType for HeaderPairRecord {
//     fn array_type_info() -> sqlx::postgres::PgTypeInfo {
//         sqlx::postgres::PgTypeInfo::with_name("header_pair")
//     }
// }

pub async fn get_saved_response(
    pool:&PgPool,
    user_id:Uuid,
    idempotency_key:&IdempotencyKey
) -> Result<Option<HttpResponse>,anyhow::Error> {

    let saved_response = sqlx::query!(
        r#"
        SELECT response_status_code as "response_status_code!",
        response_headers as "response_headers!: Vec<HeaderPairRecord>",
        response_body as "response_body!"
        FROM idempotency
        WHERE 
        user_id = $1 AND
        idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    ).fetch_optional(pool)
        .await?;

    if let Some(record) = saved_response {
        let status_code = StatusCode::from_u16(
            record.response_status_code.try_into()?
        )?;
        let mut response = HttpResponse::build(status_code);
        for HeaderPairRecord {name,value} in record.response_headers {
            response.append_header((name,value));
        }
        Ok(Some(response.body(record.response_body)))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut transaction:Transaction<'static,Postgres>,
    user_id:Uuid,
    http_response:HttpResponse,
    idempotency_key:&IdempotencyKey
) -> Result<HttpResponse, anyhow::Error> {
    let (res,body) = http_response.into_parts();
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{}",e))?;
    let status_code = res.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(res.headers().len());
        for(name,value) in res.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord {name,value});
        }
        h
    };

    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    ).execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    let http_response = res.set_body(body).map_into_boxed_body();
    Ok(http_response)


}


