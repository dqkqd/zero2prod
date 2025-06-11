pub mod configuration;
pub mod domain;
pub mod routes;
mod startup;
pub mod telemetry;

pub use startup::{app, run};
