# eth_estimateGas

## Metadata

- name: eth_estimateGas
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/execute.yaml#L16)

## Description

Generates and returns an estimate of how much gas is necessary to allow the
transaction to complete.

Kakarot Specificity:

- Call the Kakarot Cairo smart contract's entrypoint: `eth_call` with the EVM
  transaction fields as arguments and get the returned `gas_used` variable. This
  value is the estimated gas needed to complete the transaction.
