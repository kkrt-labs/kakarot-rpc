// Utils
import { NULL_BLOCK_HASH, padBigint, padBytes } from "../utils/hex.ts";

// Starknet
import { Transaction, TransactionReceipt, uint256 } from "../deps.ts";

// Eth
import {
  AccessListEIP2930Transaction,
  bigIntToBytes,
  bigIntToHex,
  Capability,
  concatBytes,
  FeeMarketEIP1559Transaction,
  intToHex,
  isAccessListEIP2930Tx,
  isFeeMarketEIP1559TxData,
  isLegacyTx,
  JsonRpcTx,
  LegacyTransaction,
  PrefixedHexString,
  RLP,
  TransactionFactory,
  TransactionType,
  TxValuesArray,
  TypedTransaction,
  TypedTxData,
} from "../deps.ts";

/**
 * @param transaction - Typed transaction to be converted.
 * @param header - The block header of the block containing the transaction.
 * @param receipt The transaction receipt of the transaction.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The transaction in the Ethereum format, or null if the transaction is invalid.
 *
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
export function toEthTx({
  transaction,
  receipt,
  blockNumber,
  blockHash,
  isPendingBlock,
}: {
  transaction: TypedTransaction;
  receipt: TransactionReceipt;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
}): (JsonRpcTx & { yParity?: string }) | null {
  const index = receipt.transactionIndex;

  if (index === undefined) {
    console.error(
      "Known bug (apibara): ⚠️ Transaction index is undefined - Transaction index will be set to 0.",
    );
  }

  const txJSON = transaction.toJSON();
  if (
    txJSON.r === undefined ||
    txJSON.s === undefined ||
    txJSON.v === undefined
  ) {
    console.error(
      `Transaction is not signed: {r: ${txJSON.r}, s: ${txJSON.s}, v: ${txJSON.v}}`,
    );
    // TODO: Ping alert webhooks
    return null;
  }
  // If the transaction is a legacy, we can calculate it from the v value.
  // v = 35 + 2 * chainId + yParity -> chainId = (v - 35) / 2
  const chainId =
    isLegacyTx(transaction) &&
    transaction.supports(Capability.EIP155ReplayProtection)
      ? bigIntToHex((BigInt(txJSON.v) - 35n) / 2n)
      : txJSON.chainId;

  const result: JsonRpcTx & { yParity?: string } = {
    blockHash: isPendingBlock ? NULL_BLOCK_HASH : blockHash,
    blockNumber,
    from: transaction.getSenderAddress().toString(),
    gas: txJSON.gasLimit!,
    gasPrice: txJSON.gasPrice ?? txJSON.maxFeePerGas!,
    maxFeePerGas: txJSON.maxFeePerGas,
    maxPriorityFeePerGas: txJSON.maxPriorityFeePerGas,
    type: intToHex(transaction.type),
    accessList: txJSON.accessList,
    chainId,
    hash: padBytes(transaction.hash(), 32),
    input: txJSON.data!,
    nonce: txJSON.nonce!,
    to: transaction.to?.toString() ?? null,
    transactionIndex: padBigint(BigInt(index ?? 0), 32),
    value: txJSON.value!,
    v: txJSON.v,
    r: txJSON.r,
    s: txJSON.s,
    maxFeePerBlobGas: txJSON.maxFeePerBlobGas,
    blobVersionedHashes: txJSON.blobVersionedHashes,
  };
  // Adding yParity for EIP-1559 and EIP-2930 transactions
  // To fit the Ethereum format, we need to add the yParity field.
  if (
    isFeeMarketEIP1559TxData(transaction) ||
    isAccessListEIP2930Tx(transaction)
  ) {
    result.yParity = txJSON.v;
  }
  return result;
}

/**
 * @param transaction - A Kakarot transaction.
 * @returns - The Typed transaction in the Ethereum format
 */
export function toTypedEthTx({
  transaction,
}: {
  transaction: Transaction;
}): TypedTransaction | null {
  const calldata = transaction.invokeV1?.calldata;
  if (!calldata) {
    console.error("No calldata");
    console.error(JSON.stringify(transaction, null, 2));
    return null;
  }
  const callArrayLen = BigInt(calldata[0]);
  // Multi-calls are not supported for now.
  if (callArrayLen !== 1n) {
    console.error(`Invalid call array length ${callArrayLen}`);
    console.error(JSON.stringify(transaction, null, 2));
    return null;
  }

  // callArrayLen <- calldata[0]
  // to <- calldata[1]
  // selector <- calldata[2];
  // dataOffset <- calldata[3]
  // dataLength <- calldata[4]
  // calldataLen <- calldata[5]
  const bytes = concatBytes(
    ...calldata.slice(6).map((x) => bigIntToBytes(BigInt(x))),
  );

  const signature = transaction.meta.signature;
  if (signature.length !== 5) {
    console.error(`Invalid signature length ${signature.length}`);
    console.error(JSON.stringify(transaction, null, 2));
    return null;
  }
  const r = uint256.uint256ToBN({ low: signature[0], high: signature[1] });
  const s = uint256.uint256ToBN({ low: signature[2], high: signature[3] });
  const v = BigInt(signature[4]);

  try {
    const ethTxUnsigned = fromSerializedData(bytes);
    return addSignature(ethTxUnsigned, r, s, v);
  } catch (e) {
    if (e instanceof Error) {
      console.error(`Invalid transaction: ${e.message}`);
    } else {
      console.error(`Unknown throw ${e}`);
      throw e;
    }
    // TODO: Ping alert webhooks
    console.error(e);
    return null;
  }
}

/**
 * @param bytes - The bytes of the rlp encoded transaction without signature.
 * For Legacy = rlp([nonce, gasprice, startgas, to, value, data, chainid, 0, 0])
 * For EIP1559 = [0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, destination, amount, data, access_list])]
 * For EIP2930 = [0x01 || rlp([chainId, nonce, gasPrice, gasLimit, to, value, data, accessList])]
 * @returns - Decoded unsigned transaction.
 * @throws - Error if the transaction is a BlobEIP4844Tx or the rlp encoding is not an array.
 */
function fromSerializedData(bytes: Uint8Array): TypedTransaction {
  const txType = bytes[0];
  if (txType <= 0x7f) {
    switch (txType) {
      case TransactionType.AccessListEIP2930:
        return AccessListEIP2930Transaction.fromSerializedTx(bytes);
      case TransactionType.FeeMarketEIP1559:
        return FeeMarketEIP1559Transaction.fromSerializedTx(bytes);
      default:
        throw new Error(`Invalid tx type: ${txType}`);
    }
  } else {
    const values = RLP.decode(bytes);
    if (!Array.isArray(values)) {
      throw new Error("Invalid serialized tx input: must be array");
    }
    // In the case of a Legacy, we need to update the chain id to be a value >= 37.
    // This is due to the fact that LegacyTransaction's constructor (used by fromValuesArray)
    // will check if v >= 37. Since we pass it [v, r, s] = [chain_id, 0, 0], we need to force
    // the chain id to be >= 37. This value will be updated during the call to addSignature.
    values[6] = bigIntToBytes(37n);
    return LegacyTransaction.fromValuesArray(
      values as TxValuesArray[TransactionType.Legacy],
    );
  }
}

/**
 * @param tx - Typed transaction to be signed.
 * @param r - Signature r value.
 * @param s - Signature s value.
 * @param v - Signature v value. In case of EIP155ReplayProtection, must include the chain ID.
 * @returns - Passed transaction with the signature added.
 * @throws - Error if the transaction is a BlobEIP4844Tx or if v param is < 35 for a
 *         LegacyTx.
 */
function addSignature(
  tx: TypedTransaction,
  r: bigint,
  s: bigint,
  v: bigint,
): TypedTransaction {
  const TypedTxData = ((): TypedTxData => {
    if (isLegacyTx(tx)) {
      if (v < 35) {
        throw new Error(`Invalid v value: ${v}`);
      }
      return LegacyTransaction.fromTxData({
        nonce: tx.nonce,
        gasPrice: tx.gasPrice,
        gasLimit: tx.gasLimit,
        to: tx.to,
        value: tx.value,
        data: tx.data,
        v,
        r,
        s,
      });
    } else if (isAccessListEIP2930Tx(tx)) {
      return AccessListEIP2930Transaction.fromTxData({
        chainId: tx.chainId,
        nonce: tx.nonce,
        gasPrice: tx.gasPrice,
        gasLimit: tx.gasLimit,
        to: tx.to,
        value: tx.value,
        data: tx.data,
        accessList: tx.accessList,
        v,
        r,
        s,
      });
    } else if (isFeeMarketEIP1559TxData(tx)) {
      return FeeMarketEIP1559Transaction.fromTxData({
        chainId: tx.chainId,
        nonce: tx.nonce,
        maxPriorityFeePerGas: tx.maxPriorityFeePerGas,
        maxFeePerGas: tx.maxFeePerGas,
        gasLimit: tx.gasLimit,
        to: tx.to,
        value: tx.value,
        data: tx.data,
        accessList: tx.accessList,
        v,
        r,
        s,
      });
    } else {
      throw new Error(`Invalid transaction type: ${tx}`);
    }
  })();

  return TransactionFactory.fromTxData(TypedTxData);
}
