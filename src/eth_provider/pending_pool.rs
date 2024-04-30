use crate::eth_provider::provider::EthDataProvider;
use lazy_static::lazy_static;
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
    // Measure start time
    let start_time = Instant::now();
    // Start an infinite loop.
    loop {
        // Call the retry_transactions method
        if let Err(err) = eth_provider.retry_transactions().await {
            tracing::error!("Error while retrying transactions: {:?}", err);
        }
        // Calculate elapsed time in milliseconds
        let elapsed_time_ms = start_time.elapsed().as_millis();
        println!("Elapsed time to retry transactions (milliseconds): {}", elapsed_time_ms);
        // pause
        sleep(Duration::from_secs(RETRY_TX_INTERVAL)).await;
    }
}
