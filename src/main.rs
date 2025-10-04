

use zero2production::configuration::get_configuration;
use zero2production::telemetry::{get_subscriber,init_subscriber};

use zero2production::startup::Application;



#[tokio::main]
async fn main() -> std::io::Result<()> {

    let configuration = get_configuration().expect("Failed to read configuration");

    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);

    let application = Application::build(configuration).await?;
    application.run_untill_stopped().await?;
    Ok(())
}


