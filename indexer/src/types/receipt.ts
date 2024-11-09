// Utils
import { padBytes } from "../utils/hex.ts";

// Constants
import { NULL_HASH } from "../constants.ts";

// Types
import { fromJsonRpcLog } from "./log.ts";
import { JsonRpcLog, JsonRpcReceipt } from "./types.ts";

// Starknet
import { Event } from "../deps.ts";

// Eth
import {
  bigIntToHex,
  Bloom,
  bytesToHex,
  generateAddress,
  hexToBytes,
  JsonRpcTx,
  Log,
  PrefixedHexString,
  TxReceipt,
} from "../deps.ts";

/**
 * @param transaction - A Ethereum transaction.
 * @param logs - A array of Ethereum logs.
 * @param event - The "transaction_executed" event.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param cumulativeGasUsed - The cumulative gas used up to this transaction.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The Ethereum receipt.
 */
export function toEthReceipt({
  transaction,
  logs,
  event,
  blockNumber,
  blockHash,
  cumulativeGasUsed,
  isPendingBlock,
}: {
  transaction: JsonRpcTx;
  logs: JsonRpcLog[];
  event: Event;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  cumulativeGasUsed: bigint;
  isPendingBlock?: boolean;
}): JsonRpcReceipt {
  // Gas used is the last piece of data in the transaction_executed event.
  // https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/accounts/eoa/library.cairo
  const gasUsed = BigInt(event.data[event.data.length - 1]);
  // Status is the second to last piece of data in the transaction_executed event.
  // https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/accounts/eoa/library.cairo
  const status = bigIntToHex(BigInt(event.data[event.data.length - 2]));
  // If there is no destination, calculate the deployed contract address.
  const contractAddress =
    transaction.to === null
      ? padBytes(
          generateAddress(
            hexToBytes(transaction.from),
            hexToBytes(transaction.nonce),
          ),
          20,
        )
      : null;

  return {
    transactionHash: transaction.hash,
    transactionIndex: bigIntToHex(BigInt(transaction.transactionIndex ?? 0)),
    blockHash: isPendingBlock ? NULL_HASH : blockHash,
    blockNumber,
    from: transaction.from,
    to: transaction.to,
    cumulativeGasUsed: bigIntToHex(cumulativeGasUsed + gasUsed),
    gasUsed: bigIntToHex(gasUsed),
    // Incorrect, should be as in EIP1559
    // min(transaction.max_priority_fee_per_gas, transaction.max_fee_per_gas - block.base_fee_per_gas)
    // effective_gas_price = priority_fee_per_gas + block.base_fee_per_gas
    // Issue is that for now we don't have access to the block base fee per gas.
    effectiveGasPrice: transaction.gasPrice,
    contractAddress: contractAddress,
    logs,
    logsBloom: logsBloom(logs.map(fromJsonRpcLog)),
    status,
    type: transaction.type,
  };
}

/**
 * @param transaction - A Ethereum transaction.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param cumulativeGasUsed - The cumulative gas used up to this transaction.
 * @param isPendingBlock - Whether the block is pending.
 * @returns - The Ethereum receipt corresponding to a reverted out of resources transaction.
 */
export function toRevertedOutOfResourcesReceipt({
  transaction,
  blockNumber,
  blockHash,
  cumulativeGasUsed,
  isPendingBlock,
}: {
  transaction: JsonRpcTx;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  cumulativeGasUsed: bigint;
  isPendingBlock?: boolean;
}): JsonRpcReceipt {
  return {
    transactionHash: transaction.hash,
    transactionIndex: bigIntToHex(BigInt(transaction.transactionIndex ?? 0)),
    blockHash: isPendingBlock ? NULL_HASH : blockHash,
    blockNumber,
    from: transaction.from,
    to: transaction.to,
    cumulativeGasUsed: bigIntToHex(cumulativeGasUsed),
    gasUsed: bigIntToHex(0n),
    // Incorrect, should be as in EIP1559
    // min(transaction.max_priority_fee_per_gas, transaction.max_fee_per_gas - block.base_fee_per_gas)
    // effective_gas_price = priority_fee_per_gas + block.base_fee_per_gas
    // Issue is that for now we don't have access to the block base fee per gas.
    effectiveGasPrice: transaction.gasPrice,
    contractAddress: null,
    logs: [],
    logsBloom: logsBloom([]),
    status: bigIntToHex(0n),
    type: transaction.type,
  };
}

/**
 * @param logs - A array of Ethereum logs.
 * @returns - The corresponding logs bloom.
 *
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
function logsBloom(logs: Log[]): string {
  const bloom = new Bloom();
  for (let i = 0; i < logs.length; i++) {
    const log = logs[i];
    // add the address
    bloom.add(log[0]);
    // add the topics
    const topics = log[1];
    for (let q = 0; q < topics.length; q++) {
      bloom.add(topics[q]);
    }
  }
  return bytesToHex(bloom.bitvector);
}

export function fromJsonRpcReceipt(receipt: JsonRpcReceipt): TxReceipt {
  const status = BigInt(receipt.status ?? "0");
  return {
    cumulativeBlockGasUsed: BigInt(receipt.cumulativeGasUsed),
    bitvector: hexToBytes(receipt.logsBloom),
    logs: receipt.logs.map(fromJsonRpcLog),
    status: status === 0n ? 0 : 1,
  };
}
