

use actix_web::{web::{self, Form}, App, FromRequest, HttpResponse, HttpServer, Responder};


use serde::{Deserialize};



pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;
pub mod domain;
pub mod email_client;

pub use startup::run;

#[derive(Deserialize)]
pub struct FormData {
    email:String,
    name:String,
}


pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}




// pub fn run(listener:TcpListener, connection: PgPool) -> Result<Server,Error> {
//     let data = web::Data::new(connection);
//     let server = HttpServer::new(move || {
//         App::new()
//             .route("/health_check", web::get().to(health_check))
//             .route("/subscriptions", web::post().to(subscribe))
//             .app_data(data.clone())
//     })
//     .listen(listener)?
//         .run();
//
//     Ok(server)
// }

