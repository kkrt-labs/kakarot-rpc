use futures::executor::block_on;
use rstest::*;

use crate::deploy_helpers::KakarotTestEnvironmentContext;

#[fixture]
pub fn kakarot_test_env_ctx() -> KakarotTestEnvironmentContext {
    // Create a new test environment with dumped state
    let with_dumped_state = true;
    block_on(async { KakarotTestEnvironmentContext::new(with_dumped_state).await })
}
