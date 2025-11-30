
use anyhow::Context;
use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret,  SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{telemetry::spawn_blocking_with_tracing};


#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: SecretString
}

#[derive(thiserror::Error,Debug)]
pub enum AuthError {
    #[error("Invalid credentials. ")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

#[tracing::instrument(
    name = "Validate users table",
    skip_all
)]
pub async fn validate_users_table(
    pool:&PgPool,
    credential:Credentials
) -> Result<uuid::Uuid,AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::new("$argon2id$v=19$m=15000,t=2,p=1$\
                                  gZiV/M1gPc22ElAH/Jh1Hw$\
                                  CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string().into_boxed_str());

    if let Some((expedted_user_id,password_hash)) = 
        get_stored_credentials(&credential.username, &pool)
            .await?
    {
        user_id = Some(expedted_user_id);
        expected_password_hash = password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(
            expected_password_hash, 
            credential.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    user_id.ok_or_else(|| {
        anyhow::anyhow!("Unknown username.")
    }
    ).map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name="Get stored credentials",
    skip_all
)]
async fn get_stored_credentials(
    username:&str,
    pool:&PgPool
) -> Result<Option<(uuid::Uuid,SecretString)>,anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, hash_password
        FROM users
        WHERE username = $1
        "#,
        username
    )
        .fetch_optional(pool)
        .await
        .context("Failed to retrieve the row / stored credentials")?
        .map(|row| (row.user_id,SecretString::new(row.hash_password.into_boxed_str())));
    Ok(row)
}

#[tracing::instrument(
    name="Verify password hash",
    skip_all
)]
fn verify_password_hash(
    expected_password_hash:SecretString,
    password_candidate:SecretString
) -> Result<(),AuthError> {

    let expected_password = PasswordHash::new(
        expected_password_hash.expose_secret().as_ref()
    )
        .map_err(|_e| AuthError::UnexpectedError(anyhow::anyhow!("Unable to create PasswordHash Instance")))?;

    Argon2::default()
        .verify_password(password_candidate.expose_secret().as_bytes(),
        &expected_password)
        .map_err(|_e| AuthError::InvalidCredentials(anyhow::anyhow!("Invalid Credentials!.")))?;

    Ok(())
}


// #[tracing::instrument(
//     name = "Altering users password",
//     skip(pool,new_credential)
// )]
// pub async fn alter_users_password(
//     pool:&PgPool,
//     new_credential:Credentials
// ) -> Result<(),ResetError>
// {
//
//     let mut expected_password_hash = SecretString::new("$argon2id$v=19$m=15000,t=2,p=1$\
//         gZiV/M1gPc22ElAH/Jh1Hw$\
//         CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string().into_boxed_str());
//     let new_password = new_credential.password.expose_secret();
//
//     let query = sqlx::query!(r#"
//
//
//         "#)
//
// }



#[tracing::instrument(
    name = "GET USERNAME FROM USER_ID",
    skip(pool,uuid)
)]
pub async fn get_username_from_uuid(
    pool:&PgPool,
    uuid:Uuid
) -> Result<String, anyhow::Error> {
    let querys = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        uuid
    ).fetch_one(pool)
        .await
        .context("Failuser_ided to perform query to retrieve username.")?;

    Ok(querys.username)

}

#[tracing::instrument(
    name = "Reset Password"
)]
pub async fn reset_password(
    pool:&PgPool,
    user_id:Uuid,
    password:SecretString
) -> Result<(), anyhow::Error> {

    let password_hash = spawn_blocking_with_tracing(move || 
        compute_password_hash(password)
        )
        .await?
        .context("Failed to compute hash_password")?;

    let _query = sqlx::query!(
        r#"
        UPDATE users
        SET hash_password = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
        .execute(pool)
        .await
        .context("Failed to reset the password!")?;
    Ok(())

}

#[tracing::instrument(
    name = "Compute password hash",
    skip(password)
)]
fn compute_password_hash(password:SecretString) ->
Result<SecretString,anyhow::Error>
{
    let salt = SaltString::generate(&mut OsRng);

    let hashed_password = Argon2::new(
        argon2::Algorithm::Argon2id, 
        argon2::Version::V0x13, 
        Params::new(15000, 2, 1, None).unwrap())
        .hash_password(password.expose_secret().as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password {}",e))?
        .to_string();

    Ok(SecretString::new(hashed_password.into_boxed_str()))

}
