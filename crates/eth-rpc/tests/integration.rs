#[cfg(test)]
mod integration_tests {
    use std::convert::TryFrom;
    use std::sync::Arc;
    use std::time::Duration;

    use ethers::contract::ContractFactory;
    use ethers::core::k256::ecdsa::SigningKey;
    use ethers::middleware::SignerMiddleware;
    use ethers::prelude::abigen;
    use ethers::providers::{Http, Middleware, Provider};
    use ethers::signers::{LocalWallet, Signer};
    use ethers::types::{BlockId, BlockNumber, TransactionReceipt, H160, H256};
    use ethers::utils::keccak256;
    use hex::FromHex;
    use kakarot_test_utils::execution::eoa::EOA;
    use kakarot_test_utils::fixtures::katana;
    use kakarot_test_utils::rpc::start_kakarot_rpc_server;
    use kakarot_test_utils::sequencer::Katana;
    use reth_primitives::U64;
    use rstest::*;

    abigen!(ERC20, "tests/contracts/ERC20/IERC20.json");

    // ⚠️ Only one test with a Katana fixture can run at a time.
    // When trying to run two tests with a Katana fixture, the second test will fail with:
    // `thread 'test_erc20' panicked at 'Failed to start the server: Os { code: 98, kind:
    // AddrInUse, message: "Address already in use" }'`
    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_erc20(#[future] katana: Katana) {
        let (server_addr, server_handle) =
            start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");
        let wallet: LocalWallet = SigningKey::from_slice(katana.eoa().private_key().as_ref())
            .expect("EOA Private Key should be used to init a LocalWallet")
            .into();

        let provider = Provider::<Http>::try_from(format!("http://localhost:{}", server_addr.port()))
            .unwrap()
            .interval(Duration::from_millis(10u64));

        // get_chainid() returns a U256, which is a [u64; 4]
        // We only need the first u64
        let chain_id = provider.get_chainid().await.unwrap().0[0];
        let client = Arc::new(SignerMiddleware::new(provider, wallet.with_chain_id(chain_id)));

        let block_number: U64 = client.get_block_number().await.unwrap();
        let params = BlockId::Number(BlockNumber::Number(block_number));
        let block = client.get_block(params).await;
        assert!(block.is_ok());

        let bytecode = include_str!("contracts/ERC20/bytecode.json");
        let bytecode: serde_json::Value = serde_json::from_str(bytecode).unwrap();
        // Deploy an ERC20
        let factory = ContractFactory::new(
            ERC20_ABI.clone(),
            ethers::types::Bytes::from_hex(bytecode["bytecode"].as_str().unwrap()).unwrap(),
            client.clone(),
        );

        let contract = factory.deploy(()).unwrap().send().await.unwrap();
        let token = ERC20::new(contract.address(), client.clone());

        // Assert initial balance is 0
        let balance = token.balance_of(katana.eoa().evm_address().unwrap().into()).call().await.unwrap();
        assert_eq!(balance, 0u64.into());

        // Mint some tokens
        let tx: TransactionReceipt = token.mint(100u64.into()).send().await.unwrap().await.unwrap().unwrap();
        let block_number: U64 = client.get_block_number().await.unwrap();

        // Assert balance is now 100
        let balance = token.balance_of(katana.eoa().evm_address().unwrap().into()).call().await.unwrap();
        assert_eq!(balance, 100u64.into());

        // Assert on the transaction receipt
        assert_eq!(tx.status, Some(1u64.into()));
        assert_eq!(tx.transaction_index, 0.into());
        assert_eq!(tx.block_number, Some(block_number));
        assert_eq!(tx.from, katana.eoa().evm_address().unwrap().into());
        assert_eq!(tx.to, Some(contract.address()));
        assert_eq!(tx.logs.len(), 1);
        assert_eq!(tx.logs[0].topics.len(), 3);
        assert_eq!(tx.logs[0].topics[0], H256::from_slice(&keccak256("Transfer(address,address,uint256)")));
        assert_eq!(tx.logs[0].topics[1], H256::zero());
        assert_eq!(tx.logs[0].topics[2], H160::from(katana.eoa().evm_address().unwrap().as_fixed_bytes()).into());
        assert_eq!(
            tx.logs[0].data,
            ethers::types::Bytes::from_hex("0x0000000000000000000000000000000000000000000000000000000000000064")
                .unwrap()
        );

        // Stop the server
        server_handle.stop().expect("Failed to stop the server");
    }
}
