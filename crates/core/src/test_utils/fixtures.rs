use futures::executor::block_on;
use rstest::*;

use crate::test_utils::deploy_helpers::KakarotTestEnvironmentContext;

#[fixture]
pub fn kakarot_test_env_ctx() -> KakarotTestEnvironmentContext {
    block_on(async { KakarotTestEnvironmentContext::from_dump_state().await })
}
