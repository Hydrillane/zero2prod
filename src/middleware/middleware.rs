use std::ops::Deref;
use std::fmt;

use actix_web::{body::MessageBody, dev::{ServiceRequest, ServiceResponse}, error::InternalError, middleware::{ErrorHandlerResponse, Next}, FromRequest, HttpMessage};
use uuid::Uuid;

use crate::{routes::{e500, see_other}, session_crate::TypedSession};


#[derive(Debug,Clone,Copy)]
pub struct UserID(Uuid);

impl fmt::Display for UserID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserID {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn reject_anonymous_users(
    mut req:ServiceRequest,
    next: Next<impl MessageBody>
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    match session.get_user_id().map_err(e500)? {
        Some(user_id) => {
            req.extensions_mut().insert(UserID(user_id));
            next.call(req).await
        },
        None => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("User is not logged in, please log in first.");
            Err(InternalError::from_response(e, response).into())
        }
    }
}

// pub async fn not_found_error_handler<B> (
//     mut res: ServiceResponse,
//     next: Next<impl MessageBody> 
// ) -> Result<ErrorHandlerResponse<B>> {
//     let (req,res) = res.into_parts();
//     let res = res.set_body(r#"{"error", "404 not foind"}"#.to_owned());
//     let res = ServiceResponse::new(req, res)
//         .map_into_boxed_body()
//         .map_into_right_body();
//     Ok(ErrorHandlerResponse::Response(res))
//
// }
