## JSON-RPC API Methods

Based on this specification:
[ethereum/execution-apis](https://github.com/ethereum/execution-apis)

### Method Implementation State

- ❌ -> TODO
- ✅ -> Implemented
- 🟡 -> Does not exactly respect the specification
- ❎ -> Unsupported method (e.g. PoW specific methods, deprecated methods, etc.)

### Contribute

The template for the method file can be found
[here](docs/contributing/method_template.md) copy it to the new method file and
edit it corresponding to the method you're implementing. All methods should be
documented in `./methods/{method}.md`

<!-- markdownlint-disable MD013 -->

| Name                                                              | Description                                                                                                                                                                                        | State |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| eth_chainId                                                       | Returns the chain ID of the current network.                                                                                                                                                       | ✅    |
| eth_syncing                                                       | Returns an object with data about the sync status or false.                                                                                                                                        | ✅    |
| [eth_coinbase](./methods/eth_coinbase.md)                         | Returns the client coinbase address.                                                                                                                                                               | ❎    |
| eth_mining                                                        | Returns true if client is actively mining new blocks.                                                                                                                                              | ❎    |
| eth_hashrate                                                      | Returns the number of hashes per second that the node is mining with.                                                                                                                              | ❎    |
| [eth_gasPrice](./methods/eth_gasPrice.md)                         | Returns the current price per gas in wei.                                                                                                                                                          | ✅    |
| eth_accounts                                                      | Returns a list of addresses owned by client.                                                                                                                                                       | ❌    |
| eth_blockNumber                                                   | Returns the number of most recent block.                                                                                                                                                           | ✅    |
| eth_getBalance                                                    | Returns the balance of the account of given address.                                                                                                                                               | ✅    |
| [eth_getStorageAt](./methods/eth_getStorageAt.md)                 | Returns the value from a storage position at a given address.                                                                                                                                      | ✅    |
| eth_getTransactionCount                                           | Returns the number of transactions sent from an address.                                                                                                                                           | ✅    |
| eth_getBlockTransactionCountByHash                                | Returns the number of transactions in a block from a block matching the given block hash.                                                                                                          | ✅    |
| eth_getBlockTransactionCountByNumber                              | Returns the number of transactions in a block matching the given block number.                                                                                                                     | ✅    |
| eth_getUncleCountByBlockHash                                      | Returns the number of uncles in a block from a block matching the given block hash.                                                                                                                | ❎    |
| eth_getUncleCountByBlockNumber                                    | Returns the number of uncles in a block from a block matching the given block number.                                                                                                              | ❎    |
| [eth_getCode](./methods/eth_getCode.md)                           | Returns code at a given address.                                                                                                                                                                   | ✅    |
| eth_sign                                                          | The sign method calculates an Ethereum specific signature with: sign(keccak256("\x19Ethereum Signed Message:\n" + len(message) + message))).                                                       | ❎    |
| eth_signTransaction                                               | Signs a transaction that can be submitted to the network at a later time using with eth_sendRawTransaction.                                                                                        | ❎    |
| eth_sendTransaction                                               | Creates a new message call transaction or a contract creation, if the data field contains code.                                                                                                      | ❎    |
| [eth_sendRawTransaction](./methods/eth_sendRawTransaction.md)     | Creates a new message call transaction or a contract creation for signed transactions.                                                                                                               | ✅    |
| [eth_call](./methods/eth_call.md)                                 | Executes a new message call immediately without creating a transaction on the blockchain.                                                                                                          | ✅    |
| [eth_estimateGas](./methods/eth_estimateGas.md)                   | Generates and returns an estimate of how much gas is necessary to allow the transaction to complete.                                                                                               | ✅    |
| eth_getBlockByHash                                                | Returns information about a block by hash.                                                                                                                                                         | ✅    |
| eth_getBlockByNumber                                              | Returns information about a block by block number.                                                                                                                                                 | ✅    |
| eth_getTransactionByHash                                          | Returns the information about a transaction requested by transaction hash.                                                                                                                         | ✅    |
| eth_getTransactionByBlockHashAndIndex                             | Returns information about a transaction by block hash and transaction index position.                                                                                                              | ✅    |
| eth_getTransactionByBlockNumberAndIndex                           | Returns information about a transaction by block number and transaction index position.                                                                                                            | ✅    |
| eth_getTransactionReceipt                                         | Returns the receipt of a transaction by transaction hash.                                                                                                                                          | ✅    |
| eth_newFilter                                                     | Creates a filter object, based on filter options, to notify when the state changes (logs). To check if the state has changed, call eth_getFilterChanges.                                           | ❌    |
| eth_newBlockFilter                                                | Creates a filter in the node, to notify when a new block arrives. To check if the state has changed, call eth_getFilterChanges.                                                                    | ❌    |
| eth_newPendingTransactionFilter                                   | Creates a filter in the node, to notify when new pending transactions arrive. To check if the state has changed, call eth_getFilterChanges.                                                        | ❌    |
| eth_uninstallFilter                                               | Uninstalls a filter with given id. Should always be called when watch is no longer needed. Additionally Filters timeout when they aren't requested with eth_getFilterChanges for a period of time. | ❌    |
| eth_getFilterChanges                                              | Polling method for a filter, which returns an array of logs which occurred since last poll.                                                                                                        | ❌    |
| eth_getFilterLogs                                                 | Returns an array of all logs matching filter with given id.                                                                                                                                        | ❌    |
| eth_getLogs                                                       | Returns an array of all logs matching a given filter object.                                                                                                                                       | ✅    |
| eth_getWork                                                       | Returns the hash of the current block, the seedHash, and the boundary condition to be met ("target").                                                                                              | ❎    |
| eth_submitWork                                                    | Used for submitting a proof-of-work solution.                                                                                                                                                      | ❎    |
| eth_createAccessList                                              | Generates an access list for a transaction.                                                                                                                                                        |       |
| [eth_maxPriorityFeePerGas](./methods/eth_maxPriorityFeePerGas.md) | Returns the current maxPriorityFeePerGas per gas in wei. This value is equal to 0.                                                                                                                 | 🟡    |
| [eth_feeHistory](./methods/eth_feeHistory.md)                     | Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.                                                                                   | 🟡    |
| eth_getProof                                                      | Returns the merkle proof for a given account and optionally some storage keys.                                                                                                                     | ✅    |

<!-- markdownlint-enable MD013 -->
