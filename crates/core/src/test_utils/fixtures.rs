use futures::executor::block_on;
use rstest::*;

use crate::test_utils::deploy_helpers::{KakarotTestEnvironmentContext, TestContext};

#[fixture]
pub fn kakarot_test_env_ctx(
    #[default(TestContext::Simple)] test_context: TestContext,
) -> KakarotTestEnvironmentContext {
    block_on(async { KakarotTestEnvironmentContext::new(test_context).await })
}
