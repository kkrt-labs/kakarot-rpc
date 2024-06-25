// Starknet
import { Transaction, TransactionReceipt } from "../deps.ts";

// Eth
import {
  JsonRpcTx,
  JsonTx,
  PrefixedHexString,
  TypedTransaction,
} from "../deps.ts";

/**
 * Represents a hexadecimal string with a `0x` prefix.
 */
export type HexString = `0x${string}`;

/**
 * Represents a request to convert a transaction to Ethereum transaction format.
 */
export interface ToEthTxRequest {
  /** An Ethereum transaction. */
  transaction: Transaction;
  /** An Ethereum receipt. */
  receipt: TransactionReceipt;
  /** The block number in which the transaction was included, as a prefixed hex string. */
  blockNumber: PrefixedHexString;
  /** The hash of the block in which the transaction was included, as a prefixed hex string. */
  blockHash: PrefixedHexString;
  /** Indicates if the transaction is in a pending block. */
  isPendingBlock: boolean;
}

/**
 * Represents an extended JSON-RPC transaction that includes additional fields.
 */
export interface ExtendedJsonRpcTx extends JsonRpcTx {
  /** The y parity of the signature. */
  yParity?: string;
  /** Indicates if the transaction is reverted due to running out of resources. */
  isRunOutOfResources?: boolean;
}

/**
 * Represents a typed transaction to Ethereum transaction conversion request.
 */
export interface TypedTxToEthTx {
  /** The typed transaction object. */
  typedTransaction: TypedTransaction;
  /** The Ethereum transaction object. */
  transaction?: Transaction;
  /** The Ethereum receipt. */
  receipt: TransactionReceipt;
  /** The block number in which the transaction was included, as a prefixed hex string. */
  blockNumber: PrefixedHexString;
  /** The hash of the block in which the transaction was included, as a prefixed hex string. */
  blockHash: PrefixedHexString;
  /** Indicates if the transaction is in a pending block. */
  isPendingBlock: boolean;
}

/**
 * Represents the format for building a transaction in Ethereum format.
 * This interface is used to construct an Ethereum formatted transaction
 * from the provided typed transaction and its JSON representation.
 */
export interface BuildTransactionEthFormat {
  /** The typed transaction object. */
  typedTransaction: TypedTransaction;
  /** The JSON representation of the transaction. */
  jsonTx: JsonTx;
  /** The Ethereum receipt. */
  receipt: TransactionReceipt;
  /** The block number in which the transaction was included, as a prefixed hex string. */
  blockNumber: PrefixedHexString;
  /** The hash of the block in which the transaction was included, as a prefixed hex string. */
  blockHash: PrefixedHexString;
  /** Indicates if the transaction is in a pending block. */
  isPendingBlock: boolean;
  /** The chain id of the transaction. */
  chainId: string | undefined;
  /** The index of the transaction in the block. */
  index: string;
}
