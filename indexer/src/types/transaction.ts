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
  hexToBytes,
  bytesToBigInt,
} from "../deps.ts";

/**
 * @param transaction - A Kakarot transaction.
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
  transaction: Transaction;
  receipt: TransactionReceipt;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
}): (JsonRpcTx & { yParity?: string; isRunOutOfResources?: boolean }) | null {
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
 * @param typeTransaction - Typed transaction to be converted.
 * @param receipt The transaction receipt of the transaction.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The transaction in the Ethereum format, or null if the transaction is invalid.
 *
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
export function typedTransactionToEthTx({
  typedTransaction,
  receipt,
  blockNumber,
  blockHash,
  isPendingBlock,
}: {
  typedTransaction: TypedTransaction;
  receipt: TransactionReceipt;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
}): (JsonRpcTx & { yParity?: string; isRunOutOfResources?: boolean }) | null {
  const index = receipt.transactionIndex;

  if (!index) {
    console.error(
      "Known bug (apibara): ⚠️ Transaction index is undefined - Transaction index will be set to 0.",
    );
  }

  const txJSON = typedTransaction.toJSON();
  if (!txJSON.r || !txJSON.s || !txJSON.v) {
    console.error(
      `Transaction is not signed: {r: ${txJSON.r}, s: ${txJSON.s}, v: ${txJSON.v}}`,
    );
    // TODO: Ping alert webhooks
    return null;
  }
  // If the transaction is a legacy, we can calculate it from the v value.
  // v = 35 + 2 * chainId + yParity -> chainId = (v - 35) / 2
  const chainId = isLegacyTx(typedTransaction) &&
      typedTransaction.supports(Capability.EIP155ReplayProtection)
    ? bigIntToHex((BigInt(txJSON.v) - 35n) / 2n)
    : txJSON.chainId;

  const result: JsonRpcTx & {
    yParity?: string;
    isRunOutOfResources?: boolean;
  } = {
    blockHash: isPendingBlock ? null : blockHash,
    blockNumber,
    from: typedTransaction.getSenderAddress().toString(), // no need to pad as the `Address` type is 40 bytes.
    gas: txJSON.gasLimit!,
    gasPrice: txJSON.gasPrice ?? txJSON.maxFeePerGas!,
    maxFeePerGas: txJSON.maxFeePerGas,
    maxPriorityFeePerGas: txJSON.maxPriorityFeePerGas,
    type: intToHex(typedTransaction.type),
    accessList: txJSON.accessList,
    chainId,
    hash: padBytes(typedTransaction.hash(), 32),
    input: txJSON.data!,
    nonce: txJSON.nonce!,
    to: typedTransaction.to?.toString() ?? null,
    transactionIndex: isPendingBlock ? null : padBigint(BigInt(index ?? 0), 8),
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
    isFeeMarketEIP1559TxData(typedTransaction) ||
    isAccessListEIP2930Tx(typedTransaction)
  ) {
    result.yParity = txJSON.v;
  }

  if (isRevertedWithOutOfResources(receipt)) {
    result.isRunOutOfResources = true;
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
  // signedDataLen <- calldata[6]
  const bytes = unpackCallData(calldata);

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
export function packCallData(input: Uint8Array): `0x${string}`[] {
  const serializedTx: `0x${string}`[] = [];

  // Process the input bytes in chunks of 31 bytes each and pack them into hexadecimal strings.
  for (let i = 0; i < input.length; i += 31) {
    // Obtain a chunk of 31 bytes.
    const chunk = input.slice(i, i + 31);

    // Convert the chunk into a BigInt, then into a hexadecimal string, padding to 64 characters.
    const hexString = ("0x" +
      bytesToBigInt(chunk).toString(16).padStart(64, "0")) as `0x${string}`;

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
export function unpackCallData(input: `0x${string}`[]): Uint8Array {
  // Convert a hex string to bytes and remove the first byte.
  const hexToBytesSlice = (x: `0x${string}`) => hexToBytes(x).slice(1);

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
