use futures::executor::block_on;
use rstest::*;

use crate::test_utils::deploy_helpers::KakarotTestEnvironmentContext;

#[fixture]
pub fn kakarot_test_env_ctx() -> KakarotTestEnvironmentContext {
    // Create a new test environment with dumped state
    block_on(async { KakarotTestEnvironmentContext::new(true).await })
}
