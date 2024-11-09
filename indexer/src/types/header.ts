// Utils
import { padString } from "../utils/hex.ts";

// Starknet
import { BlockHeader } from "../deps.ts";

// Eth
import { bigIntToHex, Bloom, bytesToHex, PrefixedHexString } from "../deps.ts";
import { JsonRpcBlock } from "./types.ts";
import { KAKAROT } from "../provider.ts";

// Constant
import { DEFAULT_BLOCK_GAS_LIMIT, NULL_HASH } from "../constants.ts";

/**
 * Converts a Starknet block header to an Ethereum block header in JSON RPC format.
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
  // Convert timestamp to Unix timestamp (seconds since Jan 1, 1970, UTC)
  const timestampUnix = Date.parse(header.timestamp);
  const timestamp = isNaN(timestampUnix)
    ? BigInt(0)
    : BigInt(timestampUnix / 1000);

  // Determine the block identifier based on whether the block is pending or finalized
  // ⚠️ StarknetJS: blockIdentifier is a block hash if value is BigInt or String, otherwise it's a block number.
  const blockIdentifier = isPendingBlock ? "pending" : blockHash;

  // Function to handle KAKAROT calls with error handling and default values
  const getResponse = async (
    method: string,
    defaultValue: bigint,
  ): Promise<bigint> => {
    try {
      // Make the KAKAROT RPC call to retrieve blockchain data
      const response = (await KAKAROT.call(method, [], {
        blockIdentifier,
      })) as {
        coinbase?: bigint;
        base_fee?: bigint;
        block_gas_limit?: bigint;
      };

      // Extract and return the specific field from the response, or fallback to default value
      switch (method) {
        case "get_coinbase":
          return BigInt(response.coinbase ?? defaultValue);
        case "get_base_fee":
          return BigInt(response.base_fee ?? defaultValue);
        case "get_block_gas_limit":
          return BigInt(response.block_gas_limit ?? defaultValue);
        default:
          return defaultValue;
      }
    } catch (error) {
      // Handle errors, log a warning, and return the default value
      console.warn(
        `⚠️ Failed to get ${method} for block ${blockNumber} - Error: ${error.message}`,
      );
      return defaultValue;
    }
  };

  // Retrieve responses for coinbase, baseFee, and blockGasLimit asynchronously
  const [coinbase, baseFee, blockGasLimit] = await Promise.all([
    getResponse("get_coinbase", BigInt(0)),
    getResponse("get_base_fee", BigInt(0)),
    getResponse("get_block_gas_limit", BigInt(DEFAULT_BLOCK_GAS_LIMIT)),
  ]);

  // Construct and return the Ethereum block header
  return {
    // Block number in hexadecimal format
    number: blockNumber,
    // Block hash or null if pending
    hash: isPendingBlock ? NULL_HASH : blockHash,
    // Padded parent block hash
    parentHash: padString(header.parentBlockHash, 32),
    // Padded mix hash (unused in this context)
    mixHash: NULL_HASH,
    // Padded nonce (unused in this context)
    nonce: padString("0x", 8),
    // Empty list of uncles -> RLP encoded to 0xC0 -> Keccak(0xc0) == 0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347
    sha3Uncles:
      "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    // Convert logs bloom filter to hexadecimal string
    logsBloom: bytesToHex(logsBloom.bitvector),
    // Convert transaction trie root to hexadecimal string
    transactionsRoot: bytesToHex(transactionRoot),
    // New state root or padded string
    stateRoot: header.newRoot ?? NULL_HASH,
    // Convert receipt trie root to hexadecimal string
    receiptsRoot: bytesToHex(receiptRoot),
    // Convert coinbase address to padded hexadecimal string
    miner: padString(bigIntToHex(coinbase), 20),
    // Difficulty field (unused in this context)
    difficulty: "0x00",
    // Total difficulty field (unused in this context)
    totalDifficulty: "0x00",
    // Extra data field (unused in this context)
    extraData: "0x",
    // Size field (unused in this context)
    size: "0x00",
    // Convert block gas limit to padded hexadecimal string
    gasLimit: padString(bigIntToHex(blockGasLimit), 32),
    // Convert total gas used to hexadecimal string
    gasUsed: bigIntToHex(gasUsed),
    // Convert timestamp to hexadecimal string
    timestamp: bigIntToHex(timestamp),
    // Empty array since transactions are not included in this representation
    transactions: [],
    // Empty array for uncles (unused in this context)
    uncles: [],
    // Empty array for withdrawals (unused in this context)
    withdrawals: [],
    // Root hash of an empty trie.
    // <https://github.com/alloy-rs/alloy/blob/e201df849552ee8e3279723de18add7ccf21e1ab/crates/consensus/src/constants.rs#L59>
    withdrawalsRoot:
      "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    // Convert base fee per gas to padded hexadecimal string
    baseFeePerGas: padString(bigIntToHex(baseFee), 32),
  };
}
