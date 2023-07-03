use lazy_static::lazy_static;
use reth_primitives::Address;
use starknet_crypto::FieldElement;

lazy_static! {
    /// Test value for Kakarot contract address.
    pub static ref KAKAROT_ADDRESS: FieldElement =
        FieldElement::from_hex_be("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",).unwrap();
    /// Test value for Abdel address.
    pub static ref ABDEL_ADDRESS: Address = Address::from_low_u64_be(0xabde1);
}
