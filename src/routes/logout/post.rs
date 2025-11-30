use actix_web::HttpResponse;
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use std::fmt::Write;

use crate::{routes::{e500, see_other}, session_crate::TypedSession};


#[tracing::instrument(
    name = "Logout got invoked!",
    skip(session)
)]
pub async fn logout(session:TypedSession,
    ) -> Result<HttpResponse,actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        Ok(see_other("/login"))
    } else {
        FlashMessage::info("You have successfully logged out.").send();
        session.logout();
        Ok(see_other("/login"))
    }
}
