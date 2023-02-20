# eth_getBlockTransactionCountByHash

## Metadata

- name: eth_getBlockTransactionCountByHash
- prefix: eth
- state: ⚠️
- [specification](https://github.com/ethereum/execution-apis/blob/main/src/eth/block.yaml#L33)
- [issue](https://github.com/sayajin-labs/kakarot-rpc/issues/61)

## Specification Description

Returns the number of transactions in a block matching the given block hash.

### Parameters

- [BlockNumberOrTagOrHash](https://github.com/ethereum/execution-apis/blob/main/src/schemas/block.yaml#L117)

### Returns

- Transaction count - Uint

## Kakarot Logic

This method does not interact with the Kakarot contract or any other Starknet
contract. It calls a Starknet JSON-RPC client and returns the number of
transactions in a block matching the given block hash.

### Starknet methods

- [starknet_getBlockTransactionCount](https://github.com/starkware-libs/starknet-specs/blob/a789ccc3432c57777beceaa53a34a7ae2f25fda0/api/starknet_api_openrpc.json#L373)

Example call:

```json
{
  "jsonrpc": "2.0",
  "method": "eth_getBlockTransactionCountByHash",
  "params": ["0x038aa13d0794e075ae207ea914e96bf565217a71d2f041960a9f28b568d2cc62"],
  "id": 0
}
```

Example responses:

```json
{
  "jsonrpc": "2.0",
  "result": "0x24",
  "id": 0
}
```