use std::{net::TcpListener, thread};

use actix_web::{middleware::from_fn,cookie::Key, dev::{Server}, web::{self}, App, HttpServer};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use actix_session::{storage::{RedisSessionStore},  SessionMiddleware};

use crate::{configuration::{DatabaseSetting, Setting}, email_client::EmailClient, health_check, middleware::reject_anonymous_users, routes::{confirm, dashboard_page, e404, home, login, login_form, logout, publish_form, publish_newsletter, reset, reset_form, subscribe}};

pub struct Application {
    pub server:Server,
    pub port:u16,
}

pub struct ApplicationBaseUrl(pub String);

impl Application {
    pub async fn build(configuration:Setting) -> Result<Application,anyhow::Error> {
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
        let server = Self::run(listener,
            connection,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri
            )
            .await?;

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

    pub async fn run(listener:TcpListener,
        connection: PgPool,
        email_client:EmailClient,
        base_url: String,
        hmac_secret: HmacSecret,
        redis_uri:SecretString
    )
        -> Result<Server,anyhow::Error> {
            let key = Key::from(hmac_secret.0.expose_secret().as_bytes());

            let message_storage = CookieMessageStore::builder(
                key.clone()
            )
                .build();
            let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await.unwrap();
            let flashmessage_framework = FlashMessagesFramework::builder(message_storage).build();
            let data = web::Data::new(connection);
            let base_url = web::Data::new(ApplicationBaseUrl(base_url));
            let email_client = web::Data::new(email_client);
            let server = HttpServer::new(move || {
                App::new()
                    .wrap(tracing_actix_web::TracingLogger::default())
                    .wrap(flashmessage_framework.clone())
                    .wrap(SessionMiddleware::new(redis_store.clone(),key.clone()))
                    .route("/health_check", web::get().to(health_check))
                    .route("/subscriptions", web::post().to(subscribe))
                    .route("/subscriptions/confirm", web::get().to(confirm))
                    .route("/login", web::get().to(login_form))
                    .route("/login", web::post().to(login))
                    .service(
                        web::scope("/admin")
                        .wrap(from_fn(reject_anonymous_users))
                        .route("/dashboard",web::get().to(dashboard_page))
                        .route("/newsletter", web::post().to(publish_newsletter))
                        .route("/newsletter",web::get().to(publish_form))
                        .route("/reset",web::get().to(reset))
                        .route("/reset", web::post().to(reset_form))
                        .route("/logout", web::post().to(logout))
                    )
                    .route("/", web::get().to(home))
                    .default_service(
                        web::route().to(e404)
                    )
                    .app_data(data.clone())
                    .app_data(web::Data::new(hmac_secret.clone()))
                    .app_data(email_client.clone())
                    .app_data(base_url.clone())
            })
            .listen(listener)?
                .run();

            Ok(server)
    }
}

#[derive(Clone,Deserialize,Debug)]
pub struct HmacSecret(pub SecretString);

pub fn get_connection_pool(configuration:&DatabaseSetting) -> PgPool {
    PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.connection_string())

}
