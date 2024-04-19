// Utils
import { NULL_BLOCK_HASH, padBytes } from "../utils/hex.ts";

// Types
import { fromJsonRpcLog, JsonRpcLog } from "./log.ts";

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
    blockHash: isPendingBlock ? NULL_BLOCK_HASH : blockHash,
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

/**
 * Acknowledgement: Code taken from <https://github.com/ethereumjs/ethereumjs-monorepo>
 */
export type JsonRpcReceipt = {
  transactionHash: string; // DATA, 32 Bytes - hash of the transaction.
  transactionIndex: string | null; // QUANTITY - integer of the transactions index position in the block.
  blockHash: string | null; // DATA, 32 Bytes - hash of the block where this transaction was in.
  blockNumber: string | null; // QUANTITY - block number where this transaction was in.
  from: string; // DATA, 20 Bytes - address of the sender.
  to: string | null; // DATA, 20 Bytes - address of the receiver. null when it's a contract creation transaction.
  cumulativeGasUsed: string; // QUANTITY  - cumulativeGasUsed is the sum of gasUsed by this specific transaction plus the gasUsed
  // in all preceding transactions in the same block.
  effectiveGasPrice: string; // QUANTITY - The final gas price per gas paid by the sender in wei.
  gasUsed: string; // QUANTITY - The amount of gas used by this specific transaction alone.
  contractAddress: string | null; // DATA, 20 Bytes - The contract address created, if the transaction was a contract creation, otherwise null.
  logs: JsonRpcLog[]; // Array - Array of log objects, which this transaction generated.
  logsBloom: string; // DATA, 256 Bytes - Bloom filter for light clients to quickly retrieve related logs.
  // It also returns either:
  type: string; // QUANTITY - integer of the transaction's type
  root?: string; // DATA, 32 bytes of post-transaction stateroot (pre Byzantium)
  status?: string; // QUANTITY, either 1 (success) or 0 (failure)
  blobGasUsed?: string; // QUANTITY, blob gas consumed by transaction (if blob transaction)
  blobGasPrice?: string; // QUAntity, blob gas price for block including this transaction (if blob transaction)
};
