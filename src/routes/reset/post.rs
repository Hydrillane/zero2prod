use core::fmt;

use actix_web::{web::{self, Form}, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use zxcvbn::{zxcvbn, Score};

use crate::{authentication::{get_username_from_uuid, reset_password, validate_users_table, AuthError, Credentials}, middleware::UserID, routes::{e500, error_chain_fmt, see_other, utils::redirect_to_reset}};


#[derive(Deserialize)]
pub struct FormData {
    old_password:SecretString,
    new_password:SecretString,
    confirm_new_password:SecretString,
}

#[tracing::instrument(
    name = "Reset Password invoked",
    skip(form,pool,user_id)
)]
pub async fn reset_form(
    form:Form<FormData>,
    pool:web::Data<PgPool>,
    user_id:web::ReqData<UserID>
) ->Result<HttpResponse,actix_web::Error> {

    let user_id = user_id.into_inner();
    // if user_id.is_nil() {
    //     FlashMessage::error("Please login first!").send();
    //     return Ok(HttpResponse::SeeOther()
    //         .insert_header(("LOCATION","/login"))
    //         .finish())
    // }

    let username = get_username_from_uuid(&pool, *user_id).await.map_err(e500)?;

    let credential = Credentials {
        username,
        password:form.0.old_password
    };


    tracing::Span::current()
        .record("username", tracing::field::display(&credential.username));

    match check_password_strength(form.0.confirm_new_password.clone()) {
        Ok(()) => {
    if let Err(e) = validate_users_table(&pool, credential.clone()).await {
        match e {
            AuthError::InvalidCredentials(e) => {
                FlashMessage::error("Wrong Password!").send();
                return Ok(HttpResponse::SeeOther()
                    .insert_header(("LOCATION","/reset"))
                    .finish())
            }
            AuthError::UnexpectedError(e) => {
                FlashMessage::error("Something went wrong").send();
                return Ok(HttpResponse::SeeOther()
                    .insert_header(("LOCATION","/reset"))
                    .finish())
            }
        }

    };
        }
        Err(validation_error) => {
            FlashMessage::error(validation_error.message).send();
            return Ok(HttpResponse::SeeOther()
                .insert_header(("LOCATION","/reset"))
                .finish())
        }

    }


    if form.0.new_password.expose_secret() != form.0.confirm_new_password.expose_secret() {
        return Ok(redirect_to_reset());
    } else {
        reset_password(&pool, *user_id, form.0.confirm_new_password).await.map_err(e500)?;
        FlashMessage::error("Your password has been changed").send();
        Ok(see_other("/admin/dashboard"))
    }
}

#[derive(thiserror::Error)]
pub enum ResetError {
    #[error("Invalid Password Error")]
    InvalidPassword(#[source] anyhow::Error),
    #[error("Something went wrong!")]
    UnexpectedError(#[from] PasswordError),
}

impl fmt::Debug for ResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(thiserror::Error)]
pub enum PasswordError {
    #[error("Password must be atleast 8 character")]
    TooShort,
    #[error("Password must contain atleast 1 special character")]
    NotContainSpecial,
    #[error("Password must atleast contain 1 Capital/UpperCase")]
    NotContainCapital,
}

impl fmt::Debug for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(Debug, Serialize)]
pub struct PasswordValidationError {
    pub message: String,
    pub suggestion: Vec<String>,
    pub score:u8
}

impl fmt::Display for PasswordValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"Password is not valid! {}",self.message)
    }
}

impl std::error::Error for PasswordValidationError {}

#[tracing::instrument(
    name = "Checking password strength",
)]
fn check_password_strength(password:SecretString) -> Result<(), PasswordValidationError>
{

    let estimate = zxcvbn(password.expose_secret(), &[]);

    let score = estimate.score();

    if score < Score::Three {
        let mut suggestions = Vec::new();
        let mut warning_message = String::new();

        if let Some(feedback) = estimate.feedback() {
            if let Some(warning) = feedback.warning() {
                warning_message = warning.to_string();
            }

            for suggestion in feedback.suggestions() {
                suggestions.push(suggestion.to_string());
            }
        }

        if warning_message.is_empty() {
            warning_message = match score {
                Score::One => "Too Weak".into(),
                Score::Two => "Weak Password".into(),
                Score::Three => "Not secure".into(),
                Score::Four => "Strong password!".into(),
                _ => "Something Went Wrong!".into()
            }
        }

        return Err(
            PasswordValidationError {
                message:warning_message,
                suggestion:suggestions,
                score: score as u8
            }
        )
    }

    Ok(())
}

