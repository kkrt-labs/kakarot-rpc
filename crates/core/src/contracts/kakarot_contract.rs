use std::sync::Arc;

use starknet::providers::Provider;

use starknet_abigen_macros::abigen_legacy;
use starknet_abigen_parser;
use starknet_crypto::FieldElement;

abigen_legacy!(KakarotCore, "./artifacts/kakarot.json");

pub struct KakarotContract<'a, P: Provider + Send + Sync + 'static> {
    pub proxy_account_class_hash: FieldElement,
    pub externally_owned_account_class_hash: FieldElement,
    pub contract_account_class_hash: FieldElement,
    pub reader: KakarotCoreReader<'a, Arc<P>>,
}

impl<P: Provider + Send + Sync + 'static> KakarotContract<'static, P> {
    pub fn new(
        proxy_account_class_hash: FieldElement,
        externally_owned_account_class_hash: FieldElement,
        contract_account_class_hash: FieldElement,
        reader: KakarotCoreReader<'static, Arc<P>>,
    ) -> Self {
        Self { proxy_account_class_hash, externally_owned_account_class_hash, contract_account_class_hash, reader }
    }
}
