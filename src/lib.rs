#![cfg_attr(not(any(test, feature = "testing")), warn(unused_crate_dependencies))]
use opentelemetry_otlp as _;
use opentelemetry_sdk as _;
use tracing_opentelemetry as _;
use tracing_subscriber as _;

pub mod config;
pub mod eth_provider;
pub mod eth_rpc;
pub mod models;
pub mod prometheus_handler;
pub mod retry;
#[cfg(feature = "testing")]
pub mod test_utils;
pub mod tracing;
pub mod alchemy_provider;
pub mod pool_provider;
