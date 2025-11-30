use core::fmt;


use actix_web::{error::InternalError, http::StatusCode, web::{self, Form}, HttpResponse, ResponseError};
use hmac::{Hmac, Mac};
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;

use crate::session_crate::TypedSession;

use actix_web_flash_messages::FlashMessage;


use crate::{authentication::{validate_users_table, AuthError, Credentials}, routes::error_chain_fmt};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse {
        let query_string = format!(
            "error={}",
            urlencoding::Encoded::new(self.to_string())
        );

        let secret = todo!();
        let hmac_tag = {
            let mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };

        HttpResponse::build(self.status_code())
            .insert_header(("LOCATION",
                    format!("/login?{query_string}&tag={hmac_tag:x}"))
            )
                .finish()

    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication Failed")]
    InvalidCredential(#[source] anyhow::Error),
    #[error("Something went wrong!")]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    name="Logging in user",
    skip(form,pool,session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form:Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession
    ) -> Result<HttpResponse,InternalError<LoginError>> {

    let credential = Credentials {
        username: form.0.username,
        password: form.0.password,

    };

    tracing::Span::current()
        .record("username", tracing::field::display(&credential.username));

        match validate_users_table(&pool, credential.clone())
            .await {
                Ok(user_id) => {
                    tracing::Span::current()
                        .record("user_id", tracing::field::display(user_id));

                    session.renew();
                    session.insert_user_id(user_id) 
                        .map_err(|e| login_fail_redirect(LoginError::UnexpectedError(e.into())))?;

                    session.insert_username(&credential.username).map_err(|e| login_fail_redirect(LoginError::UnexpectedError(e.into())))?;

                    let response = HttpResponse::SeeOther()
                        .insert_header((
                                "LOCATION",
                                "/admin/dashboard"
                        )).finish();
                    FlashMessage::success(format!("Successfully logged in as {}",&credential.username)).send();
                    Ok(response)
                }
                Err(e) => {
                    let e = match e {
                        AuthError::InvalidCredentials(_) => LoginError::InvalidCredential(e.into()),
                        AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into())
                    };
                    Err(login_fail_redirect(e.into()))
                        }
                        }
}


fn login_fail_redirect(error:LoginError) -> InternalError<LoginError> {
    FlashMessage::error(error.to_string()).send();

    let response = HttpResponse::SeeOther()
        .insert_header(("LOCATION", "/login"))
        .finish();

    InternalError::from_response(error, response)
}
