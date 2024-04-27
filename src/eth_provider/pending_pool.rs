use crate::eth_provider::provider::EthDataProvider;
use eyre::Result;
use tokio::time::{sleep, Duration};

pub async fn start_retry_service<SP>(eth_provider: EthDataProvider<SP>) -> Result<()>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    // Start an infinite loop.
    loop {
        // Call the retry_transactions method
        if let Err(err) = eth_provider.retry_transactions().await {
            return Err(eyre::eyre!("Error while retrying transactions: {:?}", err));
        }
        // 30-second pause
        sleep(Duration::from_secs(30)).await;
    }
}
