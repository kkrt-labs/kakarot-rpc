use ethers::signers::LocalWallet;
use ethers::types::Address as EthersAddress;
use lazy_static::lazy_static;

pub const EOA_PRIVATE_KEY: &str = "024b7c9e8f15432309db022c54d3279d9b421275533e090aa03cbf4211670823";

lazy_static! {
    pub static ref EOA_WALLET: LocalWallet = EOA_PRIVATE_KEY.parse().unwrap();
    pub static ref ETH_ZERO_ADDRESS: EthersAddress =
        "0x0000000000000000000000000000000000000000".parse::<EthersAddress>().unwrap();
}
