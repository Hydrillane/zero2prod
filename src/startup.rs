use std::{io::Error, net::TcpListener};

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;

use crate::{health_check, routes::subscribe};


pub fn run(listener:TcpListener, connection: PgPool) -> Result<Server,Error> {
    let data = web::Data::new(connection);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(data.clone())
    })
    .listen(listener)?
        .run();

    Ok(server)
}

