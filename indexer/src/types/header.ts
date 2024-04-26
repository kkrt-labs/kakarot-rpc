// Utils
import { padString } from "../utils/hex.ts";

// Starknet
import { BlockHeader } from "../deps.ts";

// Eth
import {
  bigIntToHex,
  Bloom,
  bytesToHex,
  JsonRpcBlock as Block,
  PrefixedHexString,
} from "../deps.ts";
import { KAKAROT } from "../provider.ts";

// A default block gas limit in case the call to get_block_gas_limit fails.
const DEFAULT_BLOCK_GAS_LIMIT = BigInt(7_000_000);

/**
 * @param header - A Starknet block header.
 * @param blockNumber - The block number of the transaction in hex.
 * @param blockHash - The block hash of the transaction in hex.
 * @param gasUsed - The total gas used in the block.
 * @param logsBloom - The logs bloom of the block.
 * @param receiptRoot - The transaction receipt trie root of the block.
 * @param transactionRoot - The transaction trie root of the block.
 * @param isPendingBlock - Whether the block is pending.
 * @returns The Ethereum block header in the json RPC format.
 *
 * Note: We return a JsonRpcBlock instead of a JsonHeader, since the
 * JsonHeader from the ethereumjs-mono repo does not follow the
 * Ethereum json RPC format for certain fields and is used as an
 * internal type.
 */
export async function toEthHeader({
  header,
  blockNumber,
  blockHash,
  gasUsed,
  logsBloom,
  receiptRoot,
  transactionRoot,
  isPendingBlock,
}: {
  header: BlockHeader;
  blockNumber: PrefixedHexString;
  blockHash: PrefixedHexString;
  gasUsed: bigint;
  logsBloom: Bloom;
  receiptRoot: Uint8Array;
  transactionRoot: Uint8Array;
  isPendingBlock: boolean;
}): Promise<JsonRpcBlock> {
  const maybeTs = Date.parse(header.timestamp);
  const ts = isNaN(maybeTs) ? 0 : Math.floor(maybeTs / 1000);

  if (header.timestamp === undefined || isNaN(maybeTs)) {
    console.error(
      `⚠️ Block timestamp is ${header.timestamp}, Date.parse of this is invalid - Block timestamp will be set to 0.`,
    );
  }

  let coinbase;
  let baseFee;
  let blockGasLimit;
  const blockIdentifier = isPendingBlock ? "pending" : blockHash;

  try {
    const response = (await KAKAROT.call("get_coinbase", [], {
      // ⚠️ StarknetJS: blockIdentifier is a block hash if value is BigInt or String, otherwise it's a block number.
      blockIdentifier,
    })) as {
      coinbase: bigint;
    };
    coinbase = response.coinbase;
  } catch (error) {
    console.warn(
      `⚠️ Failed to get coinbase for block ${blockNumber} - Error: ${error.message}`,
    );
    coinbase = BigInt(0);
  }

  try {
    const response = (await KAKAROT.call("get_base_fee", [], {
      // ⚠️ StarknetJS: blockIdentifier is a block hash if value is BigInt or String, otherwise it's a block number.
      blockIdentifier,
    })) as {
      base_fee: bigint;
    };
    baseFee = response.base_fee;
  } catch (error) {
    console.warn(
      `⚠️ Failed to get base fee for block ${blockNumber} - Error: ${error.message}`,
    );
    baseFee = BigInt(0);
  }

  try {
    const response = (await KAKAROT.call("get_block_gas_limit", [], {
      // ⚠️ StarknetJS: blockIdentifier is a block hash if value is BigInt or String, otherwise it's a block number.
      blockIdentifier,
    })) as {
      block_gas_limit: bigint;
    };
    blockGasLimit = response.block_gas_limit;
  } catch (error) {
    console.warn(
      `⚠️ Failed to get block gas limit for block ${blockNumber} - Error: ${error.message}`,
    );
    blockGasLimit = DEFAULT_BLOCK_GAS_LIMIT;
  }

  return {
    number: blockNumber,
    hash: isPendingBlock ? null : blockHash,
    parentHash: padString(header.parentBlockHash, 32),
    mixHash: padString("0x", 32),
    nonce: padString("0x", 8),
    // Empty list of uncles -> RLP encoded to 0xC0 -> Keccak(0xc0) == 0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347
    sha3Uncles:
      "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    logsBloom: bytesToHex(logsBloom.bitvector),
    transactionsRoot: bytesToHex(transactionRoot),
    stateRoot: header.newRoot ?? padString("0x", 32),
    receiptsRoot: bytesToHex(receiptRoot),
    miner: padString(bigIntToHex(coinbase), 20),
    difficulty: "0x00",
    totalDifficulty: "0x00",
    extraData: "0x",
    size: "0x00",
    gasLimit: padString(bigIntToHex(blockGasLimit), 32),
    gasUsed: bigIntToHex(gasUsed),
    timestamp: bigIntToHex(BigInt(ts)),
    transactions: [], // we are using this structure to represent a Kakarot block header, so we don't need to include transactions
    uncles: [],
    withdrawals: [],
    // Root hash of an empty trie.
    // <https://github.com/paradigmxyz/reth/blob/main/crates/primitives/src/constants/mod.rs#L138>
    withdrawalsRoot:
      "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    baseFeePerGas: padString(bigIntToHex(baseFee), 32),
  };
}

export type JsonRpcBlock = Omit<Block, "hash"> & {
  hash: string | null;
};
