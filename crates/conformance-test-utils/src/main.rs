use clap::Parser;
use conformance_test_utils::utils::compute_starknet_address;
use starknet::core::types::FieldElement;

#[derive(Parser)]
pub struct ComputeStarknetAddressArgs {
    #[arg(long)]
    #[arg(short = 'k')]
    #[arg(help = "Set the kakarot address.")]
    pub kakarot_address: FieldElement,
    #[arg(long)]
    #[arg(short = 'a')]
    #[arg(help = "Set the account proxy class hash.")]
    pub account_proxy_class_hash: FieldElement,
    #[arg(long)]
    #[arg(short = 'e')]
    #[arg(help = "Set the EVM address.")]
    evm_address: FieldElement,
}

fn main() {
    let args = ComputeStarknetAddressArgs::parse();

    // Compute StarkNet address
    let starknet_address =
        compute_starknet_address(args.kakarot_address, args.account_proxy_class_hash, args.evm_address);

    // Convert FieldElement to Hex String
    let starknet_address: Vec<String> =
        starknet_address.to_bytes_be().iter().map(|byte| format!("{:02x}", byte)).collect();
    let starknet_address = starknet_address.join("");

    println!("Starknet Address: 0x{}", starknet_address);
}
