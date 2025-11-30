
use actix_web::{cookie::Cookie, http::header::ContentType,HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use hmac::Mac;
use secrecy::ExposeSecret;
use serde::Deserialize;
use std::fmt::Write;

use crate::startup::HmacSecret;

#[derive(Deserialize)]
pub struct QueryParameter {
    pub error: String,
    pub tag: String,
}

impl QueryParameter {
    fn verify(self,secret:
        &HmacSecret)
        -> Result<String,anyhow::Error> {
            let tag = hex::decode(self.tag)?;
            let query_errory = format!(
                "error={}",
                urlencoding::Encoded::new(&self.error)
            );

            let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(
                secret.0.expose_secret().as_bytes()
            ).unwrap();

            mac.update(&query_errory.as_bytes());
            mac.verify_slice(&tag)?;
            Ok(self.error)
    }
}

pub async fn login_form(flash_message:IncomingFlashMessages
    ) -> HttpResponse {
    let mut messages = String::new();

    for m in flash_message.iter() {
        writeln!(messages, "<p><i>{}</i></p>",m.content()).unwrap();
    }

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Login</title>
                </head>
                <body>
                {messages}
                <form action="/login" method="post">
                <label>Username
                <input
                type="text"
                placeholder="Enter Username"
                name="username"
                >
                </label>
                <label>Password
                <input
                type="password"
                placeholder="Enter Password"
                name="password"
                >
                </label>
                <button type="submit">Login</button>
                </form>
                </body></html>"#,
    ));

    response
        .add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();
        response

}
