use reth_primitives::{AccessList, AccessListItem, TransactionKind, TxEip1559, TxEip2930, TxLegacy, TxType};

use super::ConversionError;
use crate::eth_provider::error::KakarotError;

pub fn rpc_transaction_to_primitive(
    rpc_transaction: reth_rpc_types::Transaction,
) -> Result<reth_primitives::Transaction, KakarotError> {
    let tx_type = match rpc_transaction.transaction_type {
        Some(transaction_type) => {
            let transaction_type: u8 = transaction_type.try_into().map_err(|_| ConversionError)?;

            transaction_type.try_into().map_err(|_| ConversionError)?
        }
        _ => return Err(ConversionError.into()),
    };

    match tx_type {
        TxType::Legacy => Ok(reth_primitives::Transaction::Legacy(TxLegacy {
            nonce: rpc_transaction.nonce.to::<u64>(),
            gas_price: rpc_transaction.gas_price.ok_or(ConversionError)?.to::<u128>(),
            gas_limit: rpc_transaction.gas.try_into().map_err(|_| ConversionError)?,
            to: rpc_transaction.to.map_or_else(|| TransactionKind::Create, TransactionKind::Call),
            value: rpc_transaction.value,
            input: rpc_transaction.input,
            chain_id: rpc_transaction.chain_id.map(|id| id.to::<u64>()),
        })),
        TxType::Eip2930 => Ok(reth_primitives::Transaction::Eip2930(TxEip2930 {
            chain_id: rpc_transaction.chain_id.ok_or(ConversionError)?.to::<u64>(),
            nonce: rpc_transaction.nonce.to::<u64>(),
            gas_price: rpc_transaction.gas_price.ok_or(ConversionError)?.to::<u128>(),
            gas_limit: rpc_transaction.gas.try_into().map_err(|_| ConversionError)?,
            to: rpc_transaction.to.map_or_else(|| TransactionKind::Create, TransactionKind::Call),
            value: rpc_transaction.value,
            access_list: AccessList(
                rpc_transaction
                    .access_list
                    .unwrap_or_default()
                    .into_iter()
                    .map(|access_list| AccessListItem {
                        address: access_list.address,
                        storage_keys: access_list.storage_keys,
                    })
                    .collect(),
            ),
            input: rpc_transaction.input,
        })),
        TxType::Eip1559 => Ok(reth_primitives::Transaction::Eip1559(TxEip1559 {
            chain_id: rpc_transaction.chain_id.ok_or(ConversionError)?.to::<u64>(),
            nonce: rpc_transaction.nonce.to::<u64>(),
            gas_limit: rpc_transaction.gas.try_into().map_err(|_| ConversionError)?,
            max_fee_per_gas: rpc_transaction.max_fee_per_gas.ok_or(ConversionError)?.to::<u128>(),
            max_priority_fee_per_gas: rpc_transaction.max_priority_fee_per_gas.ok_or(ConversionError)?.to::<u128>(),
            to: rpc_transaction.to.map_or_else(|| TransactionKind::Create, TransactionKind::Call),
            value: rpc_transaction.value,
            access_list: AccessList(
                rpc_transaction
                    .access_list
                    .unwrap_or_default()
                    .into_iter()
                    .map(|access_list| AccessListItem {
                        address: access_list.address,
                        storage_keys: access_list.storage_keys,
                    })
                    .collect(),
            ),
            input: rpc_transaction.input,
        })),
        _ => Err(ConversionError.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_primitives::{Address, Bytes, B256, U128, U256, U64};
    use reth_rpc_types::AccessListItem as RpcAccessListItem;

    use super::*;

    // Helper to create a common base for RPC transactions
    fn base_rpc_transaction() -> reth_rpc_types::Transaction {
        reth_rpc_types::Transaction {
            hash: B256::default(),
            nonce: U64::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            to: Some(Address::from_str("0x0000000000000000000000000000000000000002").unwrap()),
            value: U256::from(100),
            gas_price: Some(U128::from(20)),
            gas: U256::from(21000),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            max_fee_per_blob_gas: None,
            input: Bytes::from("1234"),
            signature: None,
            chain_id: Some(U64::from(1)),
            blob_versioned_hashes: vec![],
            access_list: None,
            transaction_type: None,
            other: serde_json::from_str("{}").unwrap(),
        }
    }

    #[test]
    fn test_legacy_transaction_conversion() {
        let mut rpc_tx = base_rpc_transaction();
        rpc_tx.transaction_type = Some(U64::from(0));

        let result = rpc_transaction_to_primitive(rpc_tx.clone());
        assert!(matches!(result, Ok(reth_primitives::Transaction::Legacy(_))));
        if let Ok(reth_primitives::Transaction::Legacy(tx)) = result {
            assert_eq!(tx.nonce, 1);
            assert_eq!(tx.gas_price, 20);
            assert_eq!(tx.gas_limit, 21000);
            assert_eq!(tx.value, U256::from(100));
            assert_eq!(tx.input, Bytes::from("1234"));
            assert_eq!(tx.chain_id, Some(1));
            assert_eq!(
                tx.to,
                TransactionKind::Call(Address::from_str("0x0000000000000000000000000000000000000002").unwrap())
            );
        }
    }

    #[test]
    fn test_eip2930_transaction_conversion() {
        let mut rpc_tx = base_rpc_transaction();
        rpc_tx.transaction_type = Some(U64::from(1));
        rpc_tx.access_list = Some(vec![RpcAccessListItem {
            address: Address::from_str("0x0000000000000000000000000000000000000003").unwrap(),
            storage_keys: vec![U256::from(123).into(), U256::from(456).into()],
        }]);

        let result = rpc_transaction_to_primitive(rpc_tx.clone());
        assert!(matches!(result, Ok(reth_primitives::Transaction::Eip2930(_))));
        if let Ok(reth_primitives::Transaction::Eip2930(tx)) = result {
            assert_eq!(tx.chain_id, 1);
            assert_eq!(tx.nonce, 1);
            assert_eq!(tx.gas_price, 20);
            assert_eq!(tx.gas_limit, 21000);
            assert_eq!(tx.value, U256::from(100));
            assert_eq!(tx.input, Bytes::from("1234"));
            assert_eq!(
                tx.to,
                TransactionKind::Call(Address::from_str("0x0000000000000000000000000000000000000002").unwrap())
            );
            assert_eq!(
                tx.access_list,
                AccessList(vec![AccessListItem {
                    address: Address::from_str("0x0000000000000000000000000000000000000003").unwrap(),
                    storage_keys: vec![U256::from(123).into(), U256::from(456).into()]
                }])
            )
        }
    }

    #[test]
    fn test_eip1559_transaction_conversion() {
        let mut rpc_tx = base_rpc_transaction();
        rpc_tx.transaction_type = Some(U64::from(2));
        rpc_tx.max_fee_per_gas = Some(U128::from(30));
        rpc_tx.max_priority_fee_per_gas = Some(U128::from(10));
        rpc_tx.access_list = Some(vec![RpcAccessListItem {
            address: Address::from_str("0x0000000000000000000000000000000000000003").unwrap(),
            storage_keys: vec![U256::from(123).into(), U256::from(456).into()],
        }]);

        let result = rpc_transaction_to_primitive(rpc_tx.clone());
        assert!(matches!(result, Ok(reth_primitives::Transaction::Eip1559(_))));
        if let Ok(reth_primitives::Transaction::Eip1559(tx)) = result {
            assert_eq!(tx.chain_id, 1);
            assert_eq!(tx.nonce, 1);
            assert_eq!(tx.max_fee_per_gas, 30);
            assert_eq!(tx.max_priority_fee_per_gas, 10);
            assert_eq!(tx.gas_limit, 21000);
            assert_eq!(tx.value, U256::from(100));
            assert_eq!(tx.input, Bytes::from("1234"));
            assert_eq!(
                tx.to,
                TransactionKind::Call(Address::from_str("0x0000000000000000000000000000000000000002").unwrap())
            );
            assert_eq!(
                tx.access_list,
                AccessList(vec![AccessListItem {
                    address: Address::from_str("0x0000000000000000000000000000000000000003").unwrap(),
                    storage_keys: vec![U256::from(123).into(), U256::from(456).into()]
                }])
            )
        }
    }

    #[test]
    #[should_panic(expected = "ConversionError")]
    fn test_invalid_transaction_type() {
        let mut rpc_tx = base_rpc_transaction();
        rpc_tx.transaction_type = Some(U64::from(99)); // Invalid type

        let _ = rpc_transaction_to_primitive(rpc_tx.clone()).unwrap();
    }
}
