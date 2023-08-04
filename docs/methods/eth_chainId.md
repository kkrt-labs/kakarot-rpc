# eth_chainId

## Metadata

- name: eth_chainId
- prefix: eth
- state: ⚠
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/client.yaml#L1)
- [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/7)

## Specification Description

Returns the chain ID of the current network.

### Parameters

- None

### Returns

- uint - chainId

## Kakarot Logic

This method does not interact with the Kakarot contract or any other Starknet
contract. The method returns a constant variable name `CHAIN_ID` that is equal
to the ASCII representation of KKRT.

### Kakarot methods

### Starknet methods
