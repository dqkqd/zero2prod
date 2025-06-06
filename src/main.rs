use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use zero2prod::{configuration::get_configuration, run, telemetry::init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_subscriber("info".into());

    let configuration = get_configuration().expect("failed to read configuration");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = tokio::net::TcpListener::bind(address).await?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&configuration.database.connection_string())
        .await
        .expect("can't connect to database");

    run(listener, pool).await
}
