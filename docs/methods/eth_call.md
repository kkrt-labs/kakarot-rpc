# eth_call

## Metadata

- name: eth_call
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/execute.yaml#L1)

## Description

Submits an EVM call by wrapping the EVM-compatible transaction object into a
Starknet call.

Kakarot Specificity:

- Call the Kakarot Cairo smart contract's entrypoint: `eth_call` with the EVM
  transaction fields as argument
