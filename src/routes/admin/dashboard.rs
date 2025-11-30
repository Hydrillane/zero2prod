use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use sqlx::PgPool;

use std::fmt::Write;

use crate::{authentication::get_username_from_uuid, routes::utils::e500, session_crate::TypedSession};

#[tracing::instrument(
    name = "Redirecting to Dashboard Page",
    skip(pool,session,flash)
)]
pub async fn dashboard_page(
    pool:web::Data<PgPool>,
    session:TypedSession,
    flash:IncomingFlashMessages
) ->Result<HttpResponse, actix_web::Error> {

    let mut message = String::new();

    for m in flash.iter() {
        writeln!(message,"<p><i>{}</i></p>",m.content()).unwrap();
    }

    let username = if let Some(user_id) = 
        session.get_user_id()
        .map_err(e500)? 
    {
        get_username_from_uuid(&pool, user_id).await.map_err(e500)?
    } 
    else {
        return Ok(HttpResponse::SeeOther()
            .insert_header(("LOCATION","/login"))
            .finish());
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
                </head>
                <body>
                {message}
                <p>Welcome {username}!</p>
                <p>Available actions:</p>
                <ol>
                <li><a href="/admin/reset">Change passwod</a></li>
                <li>
                <form name="logoutForm" action="/admin/logout" method="post">
                <input type="submit" value="Logout">
                </form>
                </li>
                <li> <a href="/admin/newsletter"> Publish Newsletter </a></li>
                </ol>
                </body>
                </html>"#
                )))
}


