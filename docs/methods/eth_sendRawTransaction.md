# eth_sendRawTransaction

## Metadata

- name: eth_sendRawTransaction
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/submit.yaml)

## Description

Submits a raw transaction by wrapping the EVM compatible transaction into a
Starknet formatted transaction. Note that this operation does not come at any
additional trust assumption. The EVM signature and initial transaction payload
will be verified inside a Cairo program (EOA Cairo implementation).

Kakarot Specificity:

- Decode RLP encoded transaction, and pass signature in the Starknet metadata
  `transaction.signature` field
- Re-encode (RLP) transaction without the signature. The encoded transaction is
  ready to be keccak-hashed inside the Cairo program (this is pre-formatting
  without security degradation).
- For a given sender EVM address, compute the corresponding (bijective mapping)
  Starknet account. Send the Starknet transaction with `sender_address` field
  set as this Starknet account.
