// Utils
import { padBigint } from "../utils/hex.ts";

// Constants
import { NULL_BLOCK_HASH } from "../constants.ts";

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

// Events containing these keys are not
// ETH logs and should be ignored.
export const IGNORED_KEYS = [
  BigInt(hash.getSelectorFromName("transaction_executed")),
  BigInt(hash.getSelectorFromName("evm_contract_deployed")),
  BigInt(hash.getSelectorFromName("Transfer")),
  BigInt(hash.getSelectorFromName("Approval")),
  BigInt(hash.getSelectorFromName("OwnershipTransferred")),
];

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
  const { keys, data } = event;
  const { transactionIndex, hash } = transaction;

  // The event must have at least one key (since the first key is the address)
  // and an odd number of keys (since each topic is split into two keys).
  // <https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/evm.cairo#L169>
  //
  // We also want to filter out ignored events which aren't ETH logs.
  if (
    keys?.length < 1 || keys.length % 2 !== 1 ||
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
    blockHash: isPendingBlock ? NULL_BLOCK_HASH : blockHash,
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
