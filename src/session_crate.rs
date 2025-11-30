use std::future::{Ready,ready};

use actix_session::{Session, SessionExt};
use actix_web::FromRequest;
use serde::ser::Error;
use uuid::Uuid;


pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";
    const USERNAME_KEY: &'static str = "username";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_id(&self, uuid:Uuid) -> Result<(), serde_json::Error> {
        self.0.insert(Self::USER_ID_KEY, uuid)
            .map_err(|e| serde_json::Error::custom(e.to_string()))
    }

    pub fn insert_username(&self,username:&String) -> Result<(), serde_json::Error> {
        self.0.insert(Self::USERNAME_KEY, username)
            .map_err(|e| serde_json::Error::custom(e.to_string()))
    }

    pub fn get_username(&self) -> Result<Option<String>,serde_json::Error>{
        self.0.get(Self::USERNAME_KEY)
            .map_err(|e| serde_json::Error::custom(e.to_string()))
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, serde_json::Error> {
        self.0.get(Self::USER_ID_KEY)
            .map_err(|e| serde_json::Error::custom(e.to_string()))
    }

    pub fn logout(&self) {
        self.0.purge();
    }

}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;

    type Future = Ready<Result<TypedSession,Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }

}
