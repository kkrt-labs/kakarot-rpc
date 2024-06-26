// Utils
import { padBigint, padBytes } from "../utils/hex.ts";
import { isRevertedWithOutOfResources } from "../utils/filter.ts";

// Starknet
import { Transaction, TransactionReceipt, uint256 } from "../deps.ts";

// Eth
import {
  AccessListEIP2930Transaction,
  bigIntToBytes,
  bigIntToHex,
  bytesToBigInt,
  Capability,
  concatBytes,
  FeeMarketEIP1559Transaction,
  hexToBytes,
  intToHex,
  isAccessListEIP2930Tx,
  isFeeMarketEIP1559TxData,
  isLegacyTx,
  JsonTx,
  LegacyTransaction,
  RLP,
  TransactionFactory,
  TransactionType,
  TxValuesArray,
  TypedTransaction,
  TypedTxData,
} from "../deps.ts";

//Interfaces
import {
  ExtendedJsonRpcTx,
  HexString,
  TransactionContext,
  TransactionConversionInput,
  TypedTransactionContext,
} from "./interfaces.ts";

/**
 * Converts a transaction to the Ethereum transaction format.
 *
 * @param transaction - The transaction object to be converted.
 * @param receipt - The transaction receipt object.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The transaction in the Ethereum format, or null if the transaction is invalid.
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
export function toEthTx({
  transaction,
  receipt,
  blockNumber,
  blockHash,
  isPendingBlock,
}: TransactionContext): ExtendedJsonRpcTx | null {
  const typedEthTx = toTypedEthTx({ transaction });
  if (!typedEthTx) {
    return null;
  }

  return typedTransactionToEthTx({
    typedTransaction: typedEthTx,
    receipt,
    blockNumber,
    blockHash,
    isPendingBlock,
  });
}

/**
 * Calculates the chain ID from the transaction.
 *
 * @param typedTransaction - The typed transaction object.
 * @param jsonTx - The JSON representation of the transaction.
 * @returns - The chain ID as a string, or undefined if the chain ID cannot be determined.
 *
 * If the transaction is a legacy transaction that supports EIP-155 replay protection,
 * the chain ID is calculated from the `v` value using the formula:
 * v = 35 + 2 * chainId + yParity -> chainId = (v - 35) / 2
 */
function chainId(
  typedTransaction: TypedTransaction,
  jsonTx: JsonTx,
): string | undefined {
  const { v, chainId } = jsonTx;

  if (
    isLegacyTx(typedTransaction) &&
    typedTransaction.supports(Capability.EIP155ReplayProtection)
  ) {
    return v
      ? bigIntToHex((BigInt(v) - 35n) / 2n)
      : (console.error("jsonTx.v is undefined"), undefined);
  }
  return chainId;
}

/**
 * Adds the yParity field to the transaction result if it is an EIP-1559 or EIP-2930 transaction.
 *
 * @param typedTransaction - The typed transaction object.
 * @param jsonTx - The JSON representation of the transaction.
 * @param result - The transaction result object in Ethereum format.
 */
function setYParityFlag(
  typedTransaction: TypedTransaction,
  jsonTx: JsonTx,
  result: ExtendedJsonRpcTx,
): void {
  // Check if the transaction is an EIP-1559 or EIP-2930 transaction
  if (
    isFeeMarketEIP1559TxData(typedTransaction) ||
    isAccessListEIP2930Tx(typedTransaction)
  ) {
    // Add the yParity field to the result
    result.yParity = jsonTx.v;
  }
}

/**
 * Adds the isRunOutOfResources flag to the transaction result if the transaction
 * was reverted due to running out of resources.
 *
 * @param receipt - The transaction receipt object.
 * @param result - The transaction result object in Ethereum format.
 */
function flagRunOutOfResources(
  receipt: TransactionReceipt,
  result: ExtendedJsonRpcTx,
): void {
  // Check if the transaction was reverted due to running out of resources
  if (isRevertedWithOutOfResources(receipt)) {
    // Set the isRunOutOfResources flag to true in the result
    result.isRunOutOfResources = true;
  }
}

/**
 * Builds the Ethereum formatted transaction from the given typed transaction and its JSON representation.
 *
 * @param typedTransaction - The typed transaction object to be converted.
 * @param jsonTx - The JSON representation of the transaction.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param isPendingBlock - Whether the block is pending.
 * @param chainId - The chain ID for the transaction.
 * @param index - The index of the transaction in the block.
 * @returns - The transaction in the Ethereum format, or null if the transaction is not signed.
 */
function transactionEthFormat({
  typedTransaction,
  jsonTx,
  blockNumber,
  blockHash,
  isPendingBlock,
  chainId,
  index,
}: TransactionConversionInput): ExtendedJsonRpcTx | null {
  if (!jsonTx.v || !jsonTx.r || !jsonTx.s) {
    console.error(
      `Transaction is not signed: {r: ${jsonTx.r}, s: ${jsonTx.s}, v: ${jsonTx.v}}`,
    );
    // TODO: Ping alert webhooks
    return null;
  }

  return {
    blockHash: isPendingBlock ? null : blockHash,
    blockNumber,
    from: typedTransaction.getSenderAddress().toString(), // no need to pad as the `Address` type is 40 bytes.
    gas: jsonTx.gasLimit!,
    gasPrice: jsonTx.gasPrice ?? jsonTx.maxFeePerGas!,
    maxFeePerGas: jsonTx.maxFeePerGas,
    maxPriorityFeePerGas: jsonTx.maxPriorityFeePerGas,
    type: intToHex(typedTransaction.type),
    accessList: jsonTx.accessList,
    chainId,
    hash: padBytes(typedTransaction.hash(), 32),
    input: jsonTx.data!,
    nonce: jsonTx.nonce!,
    to: typedTransaction.to?.toString() ?? null,
    transactionIndex: isPendingBlock ? null : padBigint(BigInt(index ?? 0), 8),
    value: jsonTx.value!,
    v: jsonTx.v,
    r: jsonTx.r,
    s: jsonTx.s,
    maxFeePerBlobGas: jsonTx.maxFeePerBlobGas,
    blobVersionedHashes: jsonTx.blobVersionedHashes,
  };
}

/**
 * Converts a typed transaction to the Ethereum transaction format.
 *
 * @param typedTransaction - Typed transaction to be converted.
 * @param receipt - The transaction receipt of the transaction.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The transaction in the Ethereum format, or null if the transaction is invalid.
 */
export function typedTransactionToEthTx({
  typedTransaction,
  receipt,
  blockNumber,
  blockHash,
  isPendingBlock,
}: TypedTransactionContext): ExtendedJsonRpcTx | null {
  const index = receipt.transactionIndex;

  if (!index) {
    console.error(
      "Known bug (apibara): ⚠️ Transaction index is undefined - Transaction index will be set to 0.",
    );
  }

  const jsonTx = typedTransaction.toJSON();

  const result = transactionEthFormat({
    typedTransaction,
    jsonTx,
    receipt,
    blockNumber,
    blockHash,
    isPendingBlock,
    chainId: chainId(typedTransaction, jsonTx),
    index,
  });

  if (!result) {
    return null;
  }

  setYParityFlag(typedTransaction, jsonTx, result);

  flagRunOutOfResources(receipt, result);

  return result;
}

/**
 * Converts a Kakarot transaction to a typed Ethereum transaction.
 *
 * @param transaction - A Kakarot transaction.
 * @returns - The Typed transaction in the Ethereum format, or null if invalid.
 */
export function toTypedEthTx({
  transaction,
}: {
  transaction: Transaction;
}): TypedTransaction | null {
  const calldata = transaction.invokeV1?.calldata;

  // Validate calldata presence
  if (!calldata) {
    console.error("No calldata", JSON.stringify(transaction, null, 2));
    return null;
  }

  // Validate single call transaction
  const callArrayLen = BigInt(calldata[0]);
  // Multi-calls are not supported for now.
  if (callArrayLen !== 1n) {
    console.error(
      `Invalid call array length ${BigInt(calldata[0])}`,
      JSON.stringify(transaction, null, 2),
    );
    return null;
  }

  // Validate signature length
  const signature = transaction.meta.signature;
  if (signature.length !== 5) {
    console.error(
      `Invalid signature length ${signature.length}`,
      JSON.stringify(transaction, null, 2),
    );
    return null;
  }

  // Extract signature components
  const [rLow, rHigh, sLow, sHigh, vBigInt] = signature;
  const r = uint256.uint256ToBN({ low: rLow, high: rHigh });
  const s = uint256.uint256ToBN({ low: sLow, high: sHigh });
  const v = BigInt(vBigInt);

  // We first try to decode the calldata in the old format, and if it fails, we try the new format.
  try {
    // Old format without 31 bytes chunks packing
    // callArrayLen <- calldata[0]
    // to <- calldata[1]
    // selector <- calldata[2];
    // dataOffset <- calldata[3]
    // dataLength <- calldata[4]
    // calldataLen <- calldata[5]
    const oldFormatBytes = concatBytes(
      ...calldata.slice(6).map((x) => bigIntToBytes(BigInt(x))),
    );

    const ethTxUnsigned = fromSerializedData(oldFormatBytes);
    return addSignature(ethTxUnsigned, r, s, v);
  } catch (_) {
    try {
      // If old format fails, try with 31 bytes chunks packing (new format)
      // callArrayLen <- calldata[0]
      // to <- calldata[1]
      // selector <- calldata[2];
      // dataOffset <- calldata[3]
      // dataLength <- calldata[4]
      // calldataLen <- calldata[5]
      // signedDataLen <- calldata[6]
      const newFormatBytes = unpackCallData(calldata);

      const ethTxUnsigned = fromSerializedData(newFormatBytes);
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
}

/**
 * Decodes an RLP encoded transaction into a TypedTransaction object.
 *
 * @param bytes - The bytes of the RLP encoded transaction without signature.
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
 * Adds a signature to a typed transaction.
 *
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

/**
 * Packs a sequence of bytes into a sequence of hexadecimal strings suitable for calldata.
 *
 * @param input - A Uint8Array containing the bytes to be packed.
 * @returns An array of hexadecimal strings, each prefixed with '0x', representing the packed calldata.
 *
 * This function processes the input bytes and packs them into chunks of 31 bytes each, converting
 * each chunk into a hexadecimal string. The resulting hexadecimal strings are padded to ensure
 * each string represents 64 hexadecimal characters (32 bytes) before being prefixed with '0x'.
 */
export function packCallData(input: Uint8Array): HexString[] {
  const serializedTx: HexString[] = [];

  // Process the input bytes in chunks of 31 bytes each and pack them into hexadecimal strings.
  for (let i = 0; i < input.length; i += 31) {
    // Obtain a chunk of 31 bytes.
    const chunk = input.slice(i, i + 31);

    // Convert the chunk into a BigInt, then into a hexadecimal string, padding to 64 characters.
    const hexString = ("0x" +
      bytesToBigInt(chunk).toString(16).padStart(64, "0")) as HexString;

    // Push the resulting hexadecimal string into the serializedTx array.
    serializedTx.push(hexString);
  }

  return serializedTx;
}

/**
 * Unpacks a sequence of hexadecimal strings representing call data into a single Uint8Array.
 *
 * @param input - An array of hexadecimal strings, each prefixed with '0x'.
 * @returns A Uint8Array containing the unpacked call data.
 *
 * This function slices and concatenates the input hexadecimal strings according to specific rules:
 *  - The elements from index 7 (inclusive and first element of signed data)
 *    to the second last element are converted from hex to bytes, sliced to remove the first byte,
 *    and concatenated.
 *  - The last element is sliced based on the remaining length required to match the byte length of calldata
 *    specified by the sixth element in the input array.
 */
export function unpackCallData(input: HexString[]): Uint8Array {
  // Convert a hex string to bytes and remove the first byte.
  const hexToBytesSlice = (x: HexString) => hexToBytes(x).slice(1);

  // Process the main part of the calldata, converting and slicing each chunk, then concatenating them.
  const calldataCore = concatBytes(...input.slice(7, -1).map(hexToBytesSlice));

  // Calculate the remaining length required to match the length specified by the sixth element in input.
  const remainingLength = parseInt(input[6], 16) - calldataCore.length;

  // Process the last element to ensure the final byte array matches the required length.
  const higher_chunk = hexToBytesSlice(input[input.length - 1]).slice(
    31 - remainingLength,
  );

  // Concatenate the processed core calldata and the adjusted last chunk into a single Uint8Array.
  return new Uint8Array([...calldataCore, ...higher_chunk]);
}
