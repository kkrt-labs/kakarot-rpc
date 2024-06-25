# eth_getStorageAt

## Metadata

- name: eth_getStorageAt
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/state.yaml#L16)

## Description

Returns the value from a storage position at a given address.

Kakarot specificity: note that Kakarot zkEVM is implemented as a set of Cairo
Programs running on an underlying StarknetOS chain (so-called CairoVM chain).

Every deployed EVM smart contract is a Starknet smart contract under the hood.
The EVM storage layout is reproduced inside the Starknet storage.

Querying a storage slot in Kakarot amounts to querying the underlying Starknet
smart contract storage using the Starknet JSON RPC specification. The
Kakarot-RPC handles this logic under-the-hood at no additional abstraction cost
or trust assumption.
