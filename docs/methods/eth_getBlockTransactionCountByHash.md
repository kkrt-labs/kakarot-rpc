# eth_getBlockTransactionCountByHash

## Metadata

- name: eth_getBlockTransactionCountByHash
- prefix: eth
- state: ⚠️
- [specification](https://github.com/ethereum/execution-apis/blob/main/src/eth/block.yaml#L33)
- [issue]()

## Specification Description

Returns the number of transactions in a block matching the given block hash.

### Parameters

- [BlockNumberOrTagOrHash](https://github.com/ethereum/execution-apis/blob/main/src/schemas/block.yaml#L117) - Block (required)

### Returns

- Transaction count - Uint

## Kakarot Logic

This method does not interact with the Kakarot contract or any other Starknet contract.
It calls a Starknet JSON-RPC client and returns the number of transactions in a block matching the given block number.

### Starknet methods

- [starknet_getBlockWithTxs](https://github.com/starkware-libs/starknet-specs/blob/master/api/starknet_api_openrpc.json#L44)
