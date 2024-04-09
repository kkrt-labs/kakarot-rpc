//! Crate providing functionalities for interacting with the Kakarot blockchain.
//!
//! The crate includes modules for configuration management, Ethereum providers and data fetching,
//! Ethereum RPC methods and utilities, models used throughout the application, handling Prometheus metrics,
//! and utilities for testing purposes.

/// Module for configurations related to the application.
pub mod config;

/// Module for Ethereum providers and data fetching.
pub mod eth_provider;

/// Module for Ethereum RPC methods and utilities.
pub mod eth_rpc;

/// Module containing models used throughout the application.
pub mod models;

/// Module for handling Prometheus metrics.
pub mod prometheus_handler;

/// Module containing utilities for testing purposes.
#[cfg(feature = "testing")]
pub mod test_utils;
