pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
mod startup;
pub mod telemetry;

pub use startup::{app, run};
