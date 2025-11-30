use actix_web::{cookie::Cookie, http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use uuid::Uuid;
use std::fmt::Write;



pub async fn publish_form(
flash_message:IncomingFlashMessages
) -> HttpResponse {

    let mut messages = String::new();

    for m in flash_message.iter() {
        write!(messages,"<p><i>{}</i></p>",m.content()).unwrap();
    }

    let idempotency_key = Uuid::new_v4().to_string();

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
                r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title> Send Newsletter Issue </title>
                <body>
                {messages}
                <form action="/admin/newsletter" method="post">
                <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
                <label> Title
                <input 
                type="text"
                placeholder="Enter Newsletter Title!"
                name="title"
                >
                </label>
                <label> Content text
                <input
                type="text"
                placeholder="Enter newsletter Content!"
                name="text_content"
                >
                </label>
                <label> Content html
                <input
                type="text"
                placeholder="Enter newsletter Content!"
                name="html_content"
                >
                </label>
                <button type="submit"> Post Newsletter </button>
                </form>
                </body></html>
                "#,
        ));

            response
            .add_removal_cookie(&Cookie::new("_flash", ""))
            .unwrap();
    response



}
