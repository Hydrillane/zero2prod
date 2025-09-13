use std::{net::{IpAddr, Ipv4Addr, SocketAddr,TcpListener}};

use once_cell::sync::Lazy;
use reqwest::Client;
use sqlx::{query::Query, Connection, PgConnection, PgPool,Executor};
use tracing::subscriber;
use uuid::Uuid;
use zero2production::{configuration::{self, DatabaseSetting}, telemetry::{get_subscriber, init_subscriber}};

use sqlx::query;

pub struct TestApp {
    address: String,
    db_connection:PgPool,
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber("test".into(), "debug".into());
    init_subscriber(subscriber);
});



#[tokio::test]
async fn health_check_works() {

    let app = spawn_app().await;

    let client = Client::new();
    let response = client
        .get(&format!("{}/health_check",&app.address))
        .send()
        .await
        .expect("Failed to execute request from client");


    assert!(response.status().is_success());
    assert_eq!(Some(0),response.content_length());

}

pub async fn spawn_app() -> TestApp {

    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address"); 
    let port = listener.local_addr().unwrap().port();

    let mut config = configuration::get_configuration().expect("Failed to read configuration");
    config.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&config.database).await;

    // let connection = PgPool::connect(&config.database.connection_string())
    //     .await
    //     .expect("Failed to connect to DB");

    let server = zero2production::run(listener,connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    let address = format!("http://127.0.0.1:{}",port);
    TestApp {
        address,
        db_connection:connection_pool
    }
}

pub async fn configure_database(config:&DatabaseSetting) -> PgPool {

    let mut connection = PgConnection::connect_with(&config.connection_without_dbname())
        .await
        .expect("Failed to connect!");

    connection
        .execute(
            format!(r#"CREATE DATABASE "{}";"#,&config.database_name).as_str())
        .await;

    let connection_pool = PgPool::connect_with(&config.connection_string())
    .await
    .expect("Failed to Connect");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to do migrations");

    connection_pool
}


#[tokio::test]
async fn subscribe_return_200_on_valid_form() {
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    let body = "name=billy%20bongso&email=billybongso2001%40gmail.com";

    let response = client
        .post(format!("{}/subscriptions",&test_app.address))
        .header("Content-Type","application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute the request");


    let saved = query!("SELECT email, name FROM subscriptions").fetch_one(&test_app.db_connection)
        .await
        .expect("Failed to fetch subscriptions");


    assert_eq!(200,response.status().as_u16());
    assert_eq!(saved.name,"billy bongso");
    assert_eq!(saved.email,"billybongso2001@gmail.com");
}


#[tokio::test]
async fn subscribe_return_400_on_invalid_form() {

    let address = spawn_app().await;

    let client = reqwest::Client::new();

    let body = vec![
        ("name=le%20guin", "missing email"),
        ("email=ursulaguin%40@gmail.com","missing name"),
        ("","missing both email and name")
    ];

    for (invalid_body,error_message) in body {
        let response = client
            .post(&format!("{}/subscriptions",&address.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "{}",
            error_message
        );
    }

}
