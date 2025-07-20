use std::fmt::{Debug, Display};

use zero2prod::{
    configuration::get_configuration, issue_delivery_worker::run_worker_until_stopped,
    startup::Application, telemetry::init_subscriber,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_subscriber(format!(
        "{}=info,tower_http=debug,axum::rejection=trace",
        env!("CARGO_CRATE_NAME")
    ));
    let configuration = get_configuration().expect("failed to read configuration");
    let application = Application::build(configuration.clone()).await?;
    let listener = tokio::net::TcpListener::bind(&application.address).await?;
    let application_task = tokio::spawn(application.run_until_stopped(listener));
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration));

    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Background worker", o),
    }
    Ok(())
}

fn report_exit(
    name: &str,
    outcome: Result<Result<(), impl Debug + Display>, tokio::task::JoinError>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                name,
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed to complete",
                name,
            )
        }
    }
}
