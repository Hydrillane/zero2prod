use std::{io::Error, net::TcpListener};

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{configuration::{DatabaseSetting, Setting}, email_client::EmailClient, health_check, routes::subscribe,routes::confirm};

pub struct Application {
    pub server:Server,
    pub port:u16,
}

pub struct ApplicationBaseUrl(pub String);

impl Application {
pub async fn build(configuration:Setting) -> Result<Application,std::io::Error> {
    let connection = PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.connection_string());

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid Sender Email address");

    let timeout = std::time::Duration::from_secs(2);

    let email_client = EmailClient::new(
        configuration.email_client.base_url, 
        sender_email, 
        configuration.email_client.authorization_token,
        timeout
        );

    let address = format!("{}:{}",configuration.application.host,configuration.application.port);

    let listener = TcpListener::bind(address)?;
    let port = listener.local_addr().unwrap().port();
    let server = run(listener,connection,email_client,configuration.application.base_url)?;

    Ok(
        Self {
            server,
            port:port
        }
    )
}

pub fn port(&self) -> u16 {
    self.port
}

pub async fn run_untill_stopped(self) -> Result<(),std::io::Error> {
    self.server.await
}

}


pub fn run(listener:TcpListener,
    connection: PgPool,
    email_client:EmailClient,
    base_url: String)
    -> Result<Server,Error> {
    let data = web::Data::new(connection);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(data.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
        .run();

    Ok(server)
}



pub fn get_connection_pool(configuration:&DatabaseSetting) -> PgPool {
    PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.connection_string())
}
