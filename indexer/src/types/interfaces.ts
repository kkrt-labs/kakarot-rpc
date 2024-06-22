// Starknet
import { Transaction, TransactionReceipt } from "../deps.ts";

// Eth
import {
  JsonRpcTx,
  PrefixedHexString,
  TypedTransaction,
  JsonTx,
} from "../deps.ts";

export type HexString = `0x${string}`

export interface ToEthTxRequest {
  transaction: Transaction;
  receipt: TransactionReceipt;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
}

export interface ExtendedJsonRpcTx extends JsonRpcTx {
  yParity?: string,
  isRunOutOfResources?: boolean 
}

export interface TypedTxToEthTx {
  typedTransaction: TypedTransaction;
  transaction?: Transaction;
  receipt: TransactionReceipt;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
}

export interface BuildTransactionEthFormat {
  typedTransaction: TypedTransaction;
  jsonTx: JsonTx;
  receipt: TransactionReceipt;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
  chainId: string | undefined;
  index: string;
}