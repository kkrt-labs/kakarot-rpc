import { encodeReceipt, RLP, TypedTransaction } from "../deps.ts";
import { fromJsonRpcReceipt, JsonRpcReceipt } from "./receipt.ts";

export type TrieData = {
  encodedTransactionIndex: Uint8Array;
  encodedTransaction: Uint8Array;
  encodedReceipt: Uint8Array;
};

export function createTrieData({
  transactionIndex,
  typedTransaction,
  receipt,
}: {
  transactionIndex: number;
  typedTransaction: TypedTransaction;
  receipt: JsonRpcReceipt;
}): TrieData {
  /// Return the eth data to be added to the tries.
  // Trie code is based off:
  // - https://github.com/ethereumjs/ethereumjs-monorepo/blob/master/packages/block/src/block.ts#L85
  // - https://github.com/ethereumjs/ethereumjs-monorepo/blob/master/packages/vm/src/buildBlock.ts#L153
  const encodedTxIndex = RLP.encode(transactionIndex);
  return {
    encodedTransactionIndex: encodedTxIndex,
    encodedTransaction: typedTransaction.serialize(),
    encodedReceipt: encodeReceipt(
      fromJsonRpcReceipt(receipt),
      typedTransaction.type,
    ),
  };
}
