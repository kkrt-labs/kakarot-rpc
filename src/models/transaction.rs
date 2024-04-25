use reth_primitives::{
    AccessList, AccessListItem, Signature, TransactionKind, TransactionSigned, TxEip1559, TxEip2930, TxLegacy, TxType,
    U256,
};

use crate::eth_provider::error::{EthApiError, EthereumDataFormatError, SignatureError, TransactionError};

pub fn rpc_to_primitive_transaction(
    rpc_transaction: reth_rpc_types::Transaction,
) -> Result<reth_primitives::Transaction, EthereumDataFormatError> {
    match rpc_transaction
        .transaction_type
        .ok_or(EthereumDataFormatError::PrimitiveError)?
        .try_into()
        .map_err(|_| EthereumDataFormatError::PrimitiveError)?
    {
        TxType::Legacy => Ok(reth_primitives::Transaction::Legacy(TxLegacy {
            nonce: rpc_transaction.nonce,
            gas_price: rpc_transaction.gas_price.ok_or(EthereumDataFormatError::PrimitiveError)?,
            gas_limit: rpc_transaction.gas.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError)?,
            to: rpc_transaction.to.map_or_else(|| TransactionKind::Create, TransactionKind::Call),
            value: rpc_transaction.value,
            input: rpc_transaction.input,
            chain_id: rpc_transaction.chain_id,
        })),
        TxType::Eip2930 => Ok(reth_primitives::Transaction::Eip2930(TxEip2930 {
            chain_id: rpc_transaction.chain_id.ok_or(EthereumDataFormatError::PrimitiveError)?,
            nonce: rpc_transaction.nonce,
            gas_price: rpc_transaction.gas_price.ok_or(EthereumDataFormatError::PrimitiveError)?,
            gas_limit: rpc_transaction.gas.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError)?,
            to: rpc_transaction.to.map_or_else(|| TransactionKind::Create, TransactionKind::Call),
            value: rpc_transaction.value,
            access_list: AccessList(
                rpc_transaction
                    .access_list
                    .unwrap_or_default()
                    .0
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
            chain_id: rpc_transaction.chain_id.ok_or(EthereumDataFormatError::PrimitiveError)?,
            nonce: rpc_transaction.nonce,
            gas_limit: rpc_transaction.gas.try_into().map_err(|_| EthereumDataFormatError::PrimitiveError)?,
            max_fee_per_gas: rpc_transaction.max_fee_per_gas.ok_or(EthereumDataFormatError::PrimitiveError)?,
            max_priority_fee_per_gas: rpc_transaction
                .max_priority_fee_per_gas
                .ok_or(EthereumDataFormatError::PrimitiveError)?,
            to: rpc_transaction.to.map_or_else(|| TransactionKind::Create, TransactionKind::Call),
            value: rpc_transaction.value,
            access_list: AccessList(
                rpc_transaction
                    .access_list
                    .unwrap_or_default()
                    .0
                    .into_iter()
                    .map(|access_list| AccessListItem {
                        address: access_list.address,
                        storage_keys: access_list.storage_keys,
                    })
                    .collect(),
            ),
            input: rpc_transaction.input,
        })),
        _ => Err(EthereumDataFormatError::PrimitiveError),
    }
}

pub fn rpc_to_ec_recovered_transaction(
    transaction: reth_rpc_types::Transaction,
) -> Result<reth_primitives::TransactionSignedEcRecovered, EthApiError> {
    let signature = transaction.signature.ok_or(SignatureError::MissingSignature)?;
    let transaction = rpc_to_primitive_transaction(transaction)?;

    let parity = match transaction.tx_type() {
        TxType::Legacy => {
            // EIP-155: v = {0, 1} + CHAIN_ID * 2 + 35
            let chain_id = transaction.chain_id().ok_or(TransactionError::InvalidChainId)?;
            let recovery: U256 = U256::from(2) * U256::from(chain_id) + U256::from(35);
            signature.v - recovery
        }
        TxType::Eip1559 | TxType::Eip2930 | TxType::Eip4844 => signature.v,
    };

    let tx_signed = TransactionSigned::from_transaction_and_signature(
        transaction,
        Signature {
            r: signature.r,
            s: signature.s,
            odd_y_parity: parity.try_into().map_err(|_| SignatureError::InvalidParity)?,
        },
    );

    let tx_ec_recovered = tx_signed.try_into_ecrecovered().map_err(|_| SignatureError::RecoveryError)?;
    Ok(tx_ec_recovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth_primitives::{Address, Bytes, U256};
    use reth_rpc_types::AccessListItem as RpcAccessListItem;
    use std::str::FromStr;

    struct RpcTxBuilder {
        tx: reth_rpc_types::Transaction,
    }

    impl RpcTxBuilder {
        fn new() -> Self {
            Self {
                tx: reth_rpc_types::Transaction {
                    nonce: 1,
                    from: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
                    to: Some(Address::from_str("0x0000000000000000000000000000000000000002").unwrap()),
                    value: U256::from(100),
                    gas_price: Some(20),
                    gas: 21000,
                    input: Bytes::from("1234"),
                    signature: Some(reth_rpc_types::Signature {
                        r: U256::from(1),
                        s: U256::from(2),
                        v: U256::from(38),
                        y_parity: None,
                    }),
                    chain_id: Some(1),
                    transaction_type: Some(0),
                    ..Default::default()
                },
            }
        }

        fn with_transaction_type(mut self, tx_type: TxType) -> Self {
            match tx_type {
                TxType::Eip2930 | TxType::Eip1559 => {
                    if let Some(sig) = self.tx.signature.as_mut() {
                        sig.v = U256::from(1);
                        sig.y_parity = Some(reth_rpc_types::Parity(true));
                    }
                }
                _ => {}
            }
            self.tx.transaction_type = Some(tx_type as u8);
            self
        }

        fn with_access_list(mut self) -> Self {
            self.tx.access_list = Some(reth_rpc_types::AccessList(vec![RpcAccessListItem {
                address: Address::from_str("0x0000000000000000000000000000000000000003").unwrap(),
                storage_keys: vec![U256::from(123).into(), U256::from(456).into()],
            }]));
            self
        }

        fn with_fee_market(mut self) -> Self {
            self.tx.max_fee_per_gas = Some(30);
            self.tx.max_priority_fee_per_gas = Some(10);
            self
        }

        fn build(self) -> reth_rpc_types::Transaction {
            self.tx
        }
    }

    // Helper to create a legacy transaction
    fn legacy_rpc_transaction() -> reth_rpc_types::Transaction {
        RpcTxBuilder::new().with_transaction_type(TxType::Legacy).build()
    }

    // Helper to create an EIP-2930 transaction
    fn eip2930_rpc_transaction() -> reth_rpc_types::Transaction {
        RpcTxBuilder::new().with_transaction_type(TxType::Eip2930).with_access_list().build()
    }

    // Helper to create an EIP-1559 transaction
    fn eip1559_rpc_transaction() -> reth_rpc_types::Transaction {
        RpcTxBuilder::new().with_transaction_type(TxType::Eip1559).with_access_list().with_fee_market().build()
    }

    macro_rules! assert_common_fields {
        ($tx: expr, $rpc_tx: expr, $gas_price_field: ident, $has_access_list: expr) => {
            assert_eq!($tx.chain_id(), $rpc_tx.chain_id);
            assert_eq!($tx.nonce(), $rpc_tx.nonce);
            assert_eq!($tx.max_fee_per_gas(), $rpc_tx.$gas_price_field.unwrap());
            assert_eq!($tx.max_priority_fee_per_gas(), $rpc_tx.max_priority_fee_per_gas);
            assert_eq!($tx.gas_limit() as u128, $rpc_tx.gas);
            assert_eq!($tx.value(), $rpc_tx.value);
            assert_eq!($tx.input().clone(), $rpc_tx.input);
            assert_eq!($tx.to().unwrap(), $rpc_tx.to.unwrap());
            if $has_access_list {
                assert_eq!(
                    $tx.access_list().cloned().unwrap(),
                    AccessList(
                        $rpc_tx
                            .access_list
                            .unwrap()
                            .0
                            .into_iter()
                            .map(|access_list| AccessListItem {
                                address: access_list.address,
                                storage_keys: access_list.storage_keys,
                            })
                            .collect()
                    )
                );
            }
        };
    }

    // Macro to create the tests for rpc to primitive transaction conversion
    macro_rules! test_rpc_to_primitive_conversion {
        ($test_name: ident, $tx_initializer: ident, $gas_price_field: ident, $has_access_list: expr) => {
            #[test]
            fn $test_name() {
                // Given
                let rpc_tx = $tx_initializer();

                // When
                let tx = rpc_to_primitive_transaction(rpc_tx.clone())
                    .expect("Failed to convert RPC transaction to ec recovered");

                // Then
                assert_common_fields!(tx, rpc_tx, $gas_price_field, $has_access_list);
            }
        };
    }

    // Macro to create the tests for rpc to ec recovered transaction conversion
    macro_rules! test_rpc_to_ec_recovered_conversion {
        ($test_name: ident, $tx_initializer: ident, $gas_price_field: ident, $has_access_list: expr) => {
            #[test]
            fn $test_name() {
                // Given
                let rpc_tx = $tx_initializer();

                // When
                let tx = rpc_to_ec_recovered_transaction(rpc_tx.clone())
                    .expect("Failed to convert RPC transaction to primitive");

                // Then
                assert_common_fields!(tx, rpc_tx, $gas_price_field, $has_access_list);
                let mut v = rpc_tx.signature.unwrap().v.to::<u64>();
                v = if v > 1 { v - tx.chain_id().unwrap() * 2 - 35 } else { v };
                assert_eq!(
                    tx.signature,
                    rpc_tx.signature.map(|sig| Signature { r: sig.r, s: sig.s, odd_y_parity: v != 0 }).unwrap()
                );
            }
        };
    }

    test_rpc_to_primitive_conversion!(
        test_legacy_transaction_conversion_to_primitive,
        legacy_rpc_transaction,
        gas_price,
        false
    );

    test_rpc_to_ec_recovered_conversion!(
        test_legacy_transaction_conversion_to_ec_recovered,
        legacy_rpc_transaction,
        gas_price,
        false
    );

    test_rpc_to_primitive_conversion!(
        test_eip2930_transaction_conversion_to_primitive,
        eip2930_rpc_transaction,
        gas_price,
        true
    );
    test_rpc_to_ec_recovered_conversion!(
        test_eip2930_transaction_conversion_to_ec_recovered,
        eip2930_rpc_transaction,
        gas_price,
        true
    );

    test_rpc_to_primitive_conversion!(
        test_eip1559_transaction_conversion_to_primitive,
        eip1559_rpc_transaction,
        max_fee_per_gas,
        true
    );

    test_rpc_to_ec_recovered_conversion!(
        test_eip1559_transaction_conversion_to_ec_recovered,
        eip1559_rpc_transaction,
        max_fee_per_gas,
        true
    );

    #[test]
    #[should_panic(expected = "PrimitiveError")]
    fn test_invalid_transaction_type() {
        let mut rpc_tx = RpcTxBuilder::new().build();
        rpc_tx.transaction_type = Some(99); // Invalid type

        let _ = rpc_to_primitive_transaction(rpc_tx).unwrap();
    }
}
