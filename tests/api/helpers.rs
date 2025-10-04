
use std::net::TcpListener;

use once_cell::sync::Lazy;
use sqlx::{Connection, PgConnection, PgPool,Executor};
use uuid::Uuid;
use wiremock::MockServer;
use zero2production::{configuration::{get_configuration, DatabaseSetting}, email_client::{self, EmailClient}, health_check, routes::subscribe, startup::get_connection_pool, telemetry::{get_subscriber, init_subscriber}};
use zero2production::startup::Application;


static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber("test".into(), "debug".into());
    init_subscriber(subscriber);
});


pub struct TestApp {
    pub address: String,
    pub db_pool:PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn post_subscriptions(&self,body:String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions",&self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let configuration = {
        let mut c = get_configuration().expect("Failed to get Configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.email_client.base_url = email_server.uri();
        c.application.port =0;
        c
    };


    configure_database(&configuration.database).await;

    let application = Application::build
        (configuration.clone())
        .await
        .expect("Failed to build app!");
    let port = application.port();

    let _ = tokio::spawn(application.run_untill_stopped());
    
    TestApp {
        address:format!("http://127.0.0.1:{}",port),
        db_pool: get_connection_pool(&configuration.database),
        email_server
    }
}


async fn configure_database(config:&DatabaseSetting) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.connection_without_dbname())
        .await
        .expect("Failed to connect!");

    connection.execute(
        format!(r#"CREATE DATABASE "{}";"#,&config.database_name).as_str())
        .await;

    let connection_pool = PgPool::connect_with(config.connection_string())
        .await
        .expect("Failed to Connect");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to do migrations");

    connection_pool
}
