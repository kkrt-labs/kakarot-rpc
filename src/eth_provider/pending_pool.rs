use crate::eth_provider::provider::EthDataProvider;
use lazy_static::lazy_static;
use std::str::FromStr;
use std::time::Instant;
use tokio::time::{sleep, Duration};

lazy_static! {
    // Interval between retries of transactions (in seconds)
    pub static ref RETRY_TX_INTERVAL: usize = usize::from_str(
        &std::env::var("RETRY_TX_INTERVAL")
            .unwrap_or_else(|_| panic!("Missing environment variable RETRY_TX_INTERVAL"))
    ).expect("failing to parse RETRY_TX_INTERVAL");
}

pub async fn start_retry_service<SP>(eth_provider: EthDataProvider<SP>)
where
    SP: starknet::providers::Provider + Send + Sync,
{
    // Initialize last print time
    let mut last_print_time = Instant::now();

    // Start an infinite loop.
    loop {
        // Measure start time
        let start_time_fn = Instant::now();
        // Call the retry_transactions method
        if let Err(err) = eth_provider.retry_transactions().await {
            tracing::error!("Error while retrying transactions: {:?}", err);
        }
        // Calculate elapsed time in milliseconds
        let elapsed_time_ms = start_time_fn.elapsed().as_millis();

        // Check if 5 minutes have passed since the last print
        if last_print_time.elapsed() >= Duration::from_secs(300) {
            tracing::info!("Elapsed time to retry transactions (milliseconds): {}", elapsed_time_ms);
            // Update last print time
            last_print_time = Instant::now();
        }

        // pause
        sleep(Duration::from_secs(*RETRY_TX_INTERVAL as u64)).await;
    }
}
