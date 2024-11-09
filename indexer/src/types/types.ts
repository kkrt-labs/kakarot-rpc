// Eth
import { JsonRpcBlock as Block, JsonRpcTx } from "../deps.ts";

/**
 * Represents a JSON-RPC block
 */
export type JsonRpcBlock = Omit<Block, "hash"> & {
  hash: string | null;
};

/**
 * Defines the possible collection types
 */
export enum Collection {
  Transactions = "transactions",
  Logs = "logs",
  Receipts = "receipts",
  Headers = "headers",
}

/**
 * Represents an item to be stored, with a generic type parameter for the collection.
 */
export type StoreItem<C = Collection> = {
  collection: C;
  data: C extends Collection.Transactions
    ? { tx: JsonRpcTx }
    : C extends Collection.Logs
      ? { log: JsonRpcLog }
      : C extends Collection.Receipts
        ? { receipt: JsonRpcReceipt }
        : { header: JsonRpcBlock };
};

/**
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
export type JsonRpcLog = {
  removed: boolean; // TAG - true when the log was removed, due to a chain reorganization. false if it's a valid log.
  logIndex: string | null; // QUANTITY - integer of the log index position in the block. null when it's pending.
  transactionIndex: string | null; // QUANTITY - integer of the transactions index position log was created from. null when it's pending.
  transactionHash: string | null; // DATA, 32 Bytes - hash of the transactions this log was created from. null when it's pending.
  blockHash: string | null; // DATA, 32 Bytes - hash of the block where this log was in. null when it's pending.
  blockNumber: string | null; // QUANTITY - the block number where this log was in. null when it's pending.
  address: string; // DATA, 20 Bytes - address from which this log originated.
  data: string; // DATA - contains one or more 32 Bytes non-indexed arguments of the log.
  topics: string[]; // Array of DATA - Array of 0 to 4 32 Bytes DATA of indexed log arguments.
  // (In solidity: The first topic is the hash of the signature of the event
  // (e.g. Deposit(address,bytes32,uint256)), except you declared the event with the anonymous specifier.)
};

/**
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
export type JsonRpcReceipt = {
  transactionHash: string; // DATA, 32 Bytes - hash of the transaction.
  transactionIndex: string | null; // QUANTITY - integer of the transactions index position in the block.
  blockHash: string | null; // DATA, 32 Bytes - hash of the block where this transaction was in.
  blockNumber: string | null; // QUANTITY - block number where this transaction was in.
  from: string; // DATA, 20 Bytes - address of the sender.
  to: string | null; // DATA, 20 Bytes - address of the receiver. null when it's a contract creation transaction.
  cumulativeGasUsed: string; // QUANTITY  - cumulativeGasUsed is the sum of gasUsed by this specific transaction plus the gasUsed
  // in all preceding transactions in the same block.
  effectiveGasPrice: string; // QUANTITY - The final gas price per gas paid by the sender in wei.
  gasUsed: string; // QUANTITY - The amount of gas used by this specific transaction alone.
  contractAddress: string | null; // DATA, 20 Bytes - The contract address created, if the transaction was a contract creation, otherwise null.
  logs: JsonRpcLog[]; // Array - Array of log objects, which this transaction generated.
  logsBloom: string; // DATA, 256 Bytes - Bloom filter for light clients to quickly retrieve related logs.
  // It also returns either:
  type: string; // QUANTITY - integer of the transaction's type
  root?: string; // DATA, 32 bytes of post-transaction stateroot (pre Byzantium)
  status?: string; // QUANTITY, either 1 (success) or 0 (failure)
  blobGasUsed?: string; // QUANTITY, blob gas consumed by transaction (if blob transaction)
  blobGasPrice?: string; // QUAntity, blob gas price for block including this transaction (if blob transaction)
};

/**
 * Represents encoded data for transaction-related tries.
 * This includes encoded versions of the transaction index,
 * the transaction itself, and its receipt.
 */
export type TrieData = {
  encodedTransactionIndex: Uint8Array;
  encodedTransaction: Uint8Array;
  encodedReceipt: Uint8Array;
};
