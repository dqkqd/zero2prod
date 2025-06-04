use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_subscriber(env_filter: String) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| env_filter.into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
