use std::fmt;

use actix_web::{HttpResponse, Responder};
use actix_web_flash_messages::FlashMessage;


pub fn e500<T>(error:T) -> actix_web::Error 
where 
    T: fmt::Debug + fmt::Display + 'static
{
    actix_web::error::ErrorInternalServerError(error)
}

pub async fn e404() -> impl Responder
{
    HttpResponse::NotFound()
        .content_type("text/html; charset=utf8")
        .body("404 - Page Not Found")
}

pub fn e400<T>(error:T) -> actix_web::Error 
where
    T: fmt::Debug + fmt::Display + 'static
{
    actix_web::error::ErrorNotFound(error)
}

pub fn redirect_to(path:&str) -> HttpResponse {
    FlashMessage::error("Please login first!").send();
    HttpResponse::SeeOther()
        .insert_header(("LOCATION",path))
        .finish()
}

pub fn redirect_to_reset() -> HttpResponse {
    FlashMessage::error("New Password doesnt match the confirmation").send();

    HttpResponse::SeeOther()
        .insert_header(("LOCATION","/reset"))
        .finish()
}

pub fn see_other(path:&str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header(("LOCATION",path))
        .finish()
}
