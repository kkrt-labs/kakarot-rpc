// Utils
import { padBigint } from "../utils/hex.ts";

// Constants
import { IGNORED_KEYS, KAKAROT_ADDRESS, NULL_HASH } from "../constants.ts";

// Types
import { JsonRpcLog } from "./types.ts";

// Starknet
import { Event, hash } from "../deps.ts";

// Eth
import {
  bigIntToHex,
  hexToBytes,
  JsonRpcTx,
  Log,
  PrefixedHexString,
} from "../deps.ts";

/**
 * @param transaction - A Ethereum transaction.
 * @param event - A Starknet event.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The log in the Ethereum format, or null if the log is invalid.
 */
export function toEthLog({
  transaction,
  event,
  blockNumber,
  blockHash,
  isPendingBlock,
}: {
  transaction: JsonRpcTx;
  event: Event;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  isPendingBlock: boolean;
}): JsonRpcLog | null {
  const { keys, data, fromAddress } = event;
  const { transactionIndex, hash } = transaction;

  // Log events originated from kakarot address only
  if (BigInt(fromAddress) !== BigInt(KAKAROT_ADDRESS)) {
    return null;
  }

  // The event must have at least one key (since the first key is the address)
  // and an odd number of keys (since each topic is split into two keys).
  // <https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/evm.cairo#L169>
  //
  // We also want to filter out ignored events which aren't ETH logs.
  if (
    keys?.length < 1 ||
    keys.length % 2 !== 1 ||
    IGNORED_KEYS.includes(BigInt(keys[0]))
  ) {
    return null;
  }

  // data field is FieldElement[] where each FieldElement represents a byte of data.
  // We convert it to a hex string and add leading zeros to make it a valid hex byte string.
  // Example: [1, 2, 3] -> "010203"
  const paddedData =
    data?.map((d) => BigInt(d).toString(16).padStart(2, "0")).join("") || "";

  // Construct topics array
  const topics: string[] = [];
  for (let i = 1; i < keys.length; i += 2) {
    // EVM Topics are u256, therefore are split into two felt keys, of at most
    // 128 bits (remember felt are 252 bits < 256 bits).
    topics[Math.floor(i / 2)] = padBigint(
      (BigInt(keys[i + 1]) << 128n) + BigInt(keys[i]),
      32,
    );
  }

  return {
    removed: false,
    logIndex: null,
    transactionIndex: bigIntToHex(BigInt(transactionIndex ?? 0)),
    transactionHash: hash,
    blockHash: isPendingBlock ? NULL_HASH : blockHash,
    blockNumber,
    // The address is the first key of the event.
    address: padBigint(BigInt(keys[0]), 20),
    data: `0x${paddedData}`,
    topics,
  };
}

/**
 * @param log - JSON RPC formatted Ethereum json rpc log.
 * @returns - A Ethereum log.
 */
export function fromJsonRpcLog(log: JsonRpcLog): Log {
  return [
    hexToBytes(log.address),
    log.topics.map(hexToBytes),
    hexToBytes(log.data),
  ];
}
