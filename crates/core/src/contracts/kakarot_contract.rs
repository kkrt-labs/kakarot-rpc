use std::sync::Arc;

use starknet::providers::Provider;

use starknet_abigen_macros::abigen_legacy;
use starknet_abigen_parser;
use starknet_crypto::FieldElement;

abigen_legacy!(KakarotCore, "./artifacts/kakarot.json");

pub struct KakarotContract<P: Provider + Send + Sync> {
    pub proxy_account_class_hash: FieldElement,
    pub externally_owned_account_class_hash: FieldElement,
    pub contract_account_class_hash: FieldElement,
    pub reader: KakarotCoreReader<Arc<P>>,
}

impl<P: Provider + Send + Sync> KakarotContract<P> {
    pub const fn new(
        proxy_account_class_hash: FieldElement,
        externally_owned_account_class_hash: FieldElement,
        contract_account_class_hash: FieldElement,
        reader: KakarotCoreReader<Arc<P>>,
    ) -> Self {
        Self { proxy_account_class_hash, externally_owned_account_class_hash, contract_account_class_hash, reader }
    }
}
