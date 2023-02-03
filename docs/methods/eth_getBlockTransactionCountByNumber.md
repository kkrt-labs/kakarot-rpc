# eth_getBlockTransactionCountByNumber

## Metadata

- name: eth_getBlockTransactionCountByNumber
- prefix: eth
- state: ⚠️
- [specification](https://github.com/ethereum/execution-apis/blob/main/src/eth/block.yaml#L43)
- [issue]()

## Specification Description

Returns the number of transactions in a block matching the given block number.

### Parameters

- [BlockNumberOrTag](https://github.com/ethereum/execution-apis/blob/main/src/schemas/block.yaml#L102) - Block (required)

### Returns

- Transaction count - Uint

## Kakarot Logic

This method does not interact with the Kakarot contract or any other Starknet contract.
It calls a Starknet JSON-RPC client and returns the number of transactions in a block matching the given block number.

### Kakarot methods

### Starknet methods

- [starknet_getBlockWithTxs](https://github.com/starkware-libs/starknet-specs/blob/master/api/starknet_api_openrpc.json#L44)

### Example

Example call:

```json
{
  "jsonrpc": "2.0",
  "method": "eth_getBlockTransactionCountByNumber",
  "params": ["latest"],
  "id": 0
}
```

Example responses:

```json

{
  "jsonrpc":"2.0",
  "result":"0x00000000000000000000000000000000000000000000000000000000000000df",
  "id": 0
}
```
