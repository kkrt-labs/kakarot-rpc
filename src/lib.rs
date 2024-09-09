#![cfg_attr(not(any(test, feature = "testing")), warn(unused_crate_dependencies))]
use opentelemetry as _;
use opentelemetry_otlp as _;
use opentelemetry_sdk as _;
use tracing_opentelemetry as _;
use tracing_subscriber as _;

pub mod providers {
    pub mod alchemy_provider;
    pub mod debug_provider;
    pub mod eth_provider;
    pub mod pool_provider;
}
pub mod client;
pub mod config;
pub mod eth_rpc;
pub mod models;
pub mod pool;
pub mod prometheus_handler;
#[cfg(feature = "testing")]
pub mod test_utils;
pub mod tracing;
