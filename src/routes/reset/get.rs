use actix_web::{cookie::Cookie, http::header::ContentType, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages};

use std::fmt::Write;

use crate::{routes::{e500, utils::{redirect_to}}, session_crate::TypedSession};

pub async fn reset(flash_message: IncomingFlashMessages, session:TypedSession) -> Result<HttpResponse,actix_web::Error> {
     if session.get_user_id().map_err(|e| e500(e))?.is_none() {
         return Ok(redirect_to("login"))
     }
    
    let mut error_html = String::new();

    for m in flash_message.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap()
    }

    let mut response =  HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(
            format!(
                r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                <meta http-equiv="content-type" content="text/html; charset="utf-8">
                <title> Change Password </title>
                <body>
                {error_html}
                <form action="/admin/reset" method="post">
                <label> Old Password
                <input
                type="password"
                placeholder="Enter old password"
                name="old_password"
                >
                </label>
                <label> New Password
                <input
                type="password"
                placeholder="Enter new password"
                name="new_password"
                >
                </label>
                </label>
                <label> Confirm New Password
                <input
                type="password"
                placeholder="Confirm new password"
                name="confirm_new_password"
                >
                </label>
                <button type="submit"> Reset </button>
                </body></html>
                "#,
    )
        );
    response.add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();
    Ok(response)
}
