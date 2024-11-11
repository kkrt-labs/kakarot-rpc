# eth_coinbase

## Metadata

- name: eth_coinbase
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/client.yaml#L15)

## Description

Returns the Ethereum account controlled by the Kakarot zkEVM sequencer.

Kakarot specificity: since Kakarot set of Cairo programs run on the StarknetOS
(i.e. on an underlying CairoVM client), the coinbase is the EVM representation
of a Starknet account that collects the fees.
