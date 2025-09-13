use std::net::TcpListener;
use sqlx::postgres::PgPoolOptions;

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
        .connect_lazy_with(configuration.database.connection_string());

    let listener = TcpListener::bind(address)?;
    run(listener,connection)?.await

}
