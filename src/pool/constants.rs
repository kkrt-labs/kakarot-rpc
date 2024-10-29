use std::time::Duration;

pub(super) static ONE_TENTH_ETH: u64 = 10u64.pow(17);

// Transactions should be pruned after 5 minutes in the mempool
pub const PRUNE_DURATION: Duration = Duration::from_secs(300);
