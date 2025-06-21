use zero2prod::{
    configuration::get_configuration, startup::Application, telemetry::init_subscriber,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber(format!(
        "{}=info,tower_http=debug,axum::rejection=trace",
        env!("CARGO_CRATE_NAME")
    ));
    let configuration = get_configuration().expect("failed to read configuration");
    let application = Application::build(configuration);
    application.run_until_stopped().await?;
    Ok(())
}
