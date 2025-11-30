

use std::fmt::{Debug, Display};

use tokio::task::JoinError;
use zero2production::configuration::get_configuration;
use zero2production::issue_delivery_work::{run_worker_until_stopped, workers_loop};
use zero2production::telemetry::{get_subscriber,init_subscriber};

use zero2production::startup::Application;



#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let configuration = get_configuration().expect("Failed to read configuration");

    let subscriber = get_subscriber("zero2prod".into(), "info".into());
    init_subscriber(subscriber);

    let application = Application::build(configuration.clone()).await?;
    let application_task = tokio::spawn(application.run_untill_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration));
    tokio::select! {
    o = application_task => report_exit("API", o),
    o = worker_task => report_exit("Background Worker", o)
    }
    Ok(())
}


fn report_exit(
    task_name:&str,
    outcome:Result<Result<(),impl Display + Debug>,JoinError>
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited",task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain=%e,
                error.message=%e,
                "{} failed",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain=%e,
                error.message=%e,
                "{} failed to complete",
                task_name
            )
        }
    }
}
