use std::net::TcpListener;
use actix_web::cookie::time::Duration;
use sqlx::postgres::PgPoolOptions;
use secrecy::ExposeSecret;

use zero2production::run;
use zero2production::configuration;
use zero2production::telemetry::{get_subscriber,init_subscriber};



#[tokio::main]
async fn main() -> std::io::Result<()> {

    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);

    let configuration = configuration::get_configuration().expect("Failed to read configuration");
    let address = format!("{}:{}",configuration.application.host,configuration.application.port);
    let connection = PgPoolOptions::new()
        .idle_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(&configuration.database.connection_string().expose_secret())
        .expect("faild to connect to DB!");

    let listener = TcpListener::bind(address)?;
    run(listener,connection)?.await

}
