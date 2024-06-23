// Starknet
import { Transaction, TransactionReceipt } from "../deps.ts";

// Eth
import {
  JsonRpcTx,
  PrefixedHexString,
  TypedTransaction,
  JsonTx,
} from "../deps.ts";

/**
 * Represents a hexadecimal string with a `0x` prefix.
 */
export type HexString = `0x${string}`

/**
 * Represents a request to convert a transaction to Ethereum transaction format.
 */
export interface ToEthTxRequest {
  transaction: Transaction; //  A Ethereum transaction.
  receipt: TransactionReceipt; // The Ethereum receipt corresponding to a reverted out of resources transaction.
  blockNumber: PrefixedHexString; // The block number in which the transaction was included, as a prefixed hex string.
  blockHash: PrefixedHexString; // The hash of the block in which the transaction was included, as a prefixed hex string.
  isPendingBlock: boolean; // Indicates if the transaction is in a pending block.
}

/**
 * Represents an extended JSON-RPC transaction that includes additional fields:
 * - yParity: The y parity of the signature.
 * - isRunOutOfResources: A flag indicating if the transaction was reverted due to running out of resources.
 */
export interface ExtendedJsonRpcTx extends JsonRpcTx {
  yParity?: string, // The y parity of the signature.
  isRunOutOfResources?: boolean // Indicates if the transaction is reverted due to running out of resources.
}

/**
 * Represents a typed transaction to Ethereum transaction conversion request.
 */
export interface TypedTxToEthTx {
  typedTransaction: TypedTransaction; // The typed transaction object.
  transaction?: Transaction; // The Ethereum transaction object.
  receipt: TransactionReceipt; // The Ethereum receipt corresponding to a reverted out of resources transaction.
  blockNumber: PrefixedHexString; // The block number in which the transaction was included, as a prefixed hex string.
  blockHash: PrefixedHexString; // The hash of the block in which the transaction was included, as a prefixed hex string.
  isPendingBlock: boolean; // Indicates if the transaction is in a pending block.
}

/**
 * Represents the format for building a transaction in Ethereum format.
 * This interface is used to construct an Ethereum formatted transaction
 * from the provided typed transaction and its JSON representation.
 */
export interface BuildTransactionEthFormat {
  typedTransaction: TypedTransaction; // The typed transaction object.
  jsonTx: JsonTx; // The JSON representation of the transaction.
  receipt: TransactionReceipt; // The Ethereum receipt corresponding to a reverted out of resources transaction.
  blockNumber: PrefixedHexString; //  The block number in which the transaction was included, as a prefixed hex string.
  blockHash: PrefixedHexString; // The hash of the block in which the transaction was included, as a prefixed hex string.
  isPendingBlock: boolean; // Indicates if the transaction is in a pending block.
  chainId: string | undefined; // The chain id of the transaction.
  index: string; // The index of the transaction in the block.
}