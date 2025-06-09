use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use zero2prod::{configuration::get_configuration, run, telemetry::init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber(format!(
        "{}=info,tower_http=debug,axum::rejection=trace",
        env!("CARGO_CRATE_NAME")
    ));

    let configuration = get_configuration().expect("failed to read configuration");
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = tokio::net::TcpListener::bind(address).await?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    run(listener, pool).await
}
