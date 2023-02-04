# eth_getTransactionReceipt

## Metadata

- name: eth_getTransactionReceipt
- prefix: eth
- state: ⚠️
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/transaction.yaml#L42)
- [issue](https://github.com/sayajin-labs/kakarot-rpc/issues/18)

## Specification Description

Returns the receipt of a transaction by transaction hash.

### Parameters

- Transaction hash

### Returns

- [Receipt Information](https://github.com/ethereum/execution-apis/blob/9500d379f872f73bcea9bc4ed21b30965099d4d7/src/schemas/receipt.yaml#L36)

## Kakarot Logic

This method does not interact with the Kakarot contract or any other Starknet contract.

### Kakarot methods

### Starknet methods

- [starknet_getTransactionReceipt](https://github.com/starkware-libs/starknet-specs/blob/df8cfb3da309f3d5dd08d804961e5a9ab8774945/api/starknet_api_openrpc.json#L215)
- [starknet_getTransactionByHash](https://github.com/starkware-libs/starknet-specs/blob/df8cfb3da309f3d5dd08d804961e5a9ab8774945/api/starknet_api_openrpc.json#L215)


### Example

Example call:

```json
{
  "jsonrpc": "2.0",
  "method": "eth_getTransactionReceipt",
  "params": ["0xb396cbe7ea9d7b9669c37fd2792a159d47c34dd72a486bb839bdfdbc0df23f90"],
  "id": 0
}
```
