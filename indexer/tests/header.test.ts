import {
  bigIntToHex,
  BlockHeader,
  Bloom,
  bytesToHex,
  PrefixedHexString,
} from "../src/deps.ts";
import { assertEquals } from "https://deno.land/std@0.213.0/assert/assert_equals.ts";
import {
  DEFAULT_BLOCK_GAS_LIMIT,
  JsonRpcBlock,
  toEthHeader,
} from "../src/types/header.ts";
import { padString } from "../src/utils/hex.ts";
import sinon from "npm:sinon";
import { KAKAROT } from "../provider.ts";
import { NULL_HASH } from "../constants.ts";

Deno.test("toEthHeader with a complete header", async () => {
  // Define a complete BlockHeader object with necessary properties
  const header: BlockHeader = {
    // Hash of the current block
    blockHash:
      "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df",
    // Hash of the parent block
    parentBlockHash:
      "0x14e330b38856cb7e21b04db7e9fd6872840333ecf753050fe421dfb42b3a3486",
    // Block number in hexadecimal
    blockNumber: "0x1",
    // Address of the sequencer
    sequencerAddress: "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97",
    // New state root
    newRoot:
      "0x3485c8e91ae6107202df98042de861c3229618b46a4bd03407edd84bafeb452f",
    // Timestamp in ISO format
    timestamp: "2023-07-03T12:34:56Z",
  };

  // Block number in hexadecimal
  const blockNumber: PrefixedHexString = "0x1";
  // Hash of the block
  const blockHash: PrefixedHexString =
    "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df";
  // Amount of gas used
  const gasUsed: bigint = BigInt(10);
  // Logs bloom filter
  const logsBloom: Bloom = new Bloom(new Uint8Array(256));
  // Receipts root
  const receiptRoot: Uint8Array = new Uint8Array(32);
  // Transactions root
  const transactionRoot: Uint8Array = new Uint8Array(33);
  // Indicate whether the block is pending
  const isPendingBlock: boolean = false;

  // Define the expected Ethereum header object
  const expectedEthHeader: JsonRpcBlock = {
    // Block number
    number: blockNumber,
    // Block hash
    hash: blockHash,
    // Parent block hash
    parentHash: header.parentBlockHash,
    // Mix hash (unused in this context)
    mixHash:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    // Nonce (unused in this context)
    nonce: "0x0000000000000000",
    // SHA3 of uncles
    sha3Uncles:
      "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    // Logs bloom in hexadecimal
    logsBloom: bytesToHex(logsBloom.bitvector),
    // Transactions root in hexadecimal
    transactionsRoot: bytesToHex(transactionRoot),
    // New state root
    stateRoot: header.newRoot,
    // Receipts root in hexadecimal
    receiptsRoot: bytesToHex(receiptRoot),
    // Miner address (defaulted to zero address)
    miner: "0x0000000000000000000000000000000000000000",
    // Difficulty (unused in this context)
    difficulty: "0x00",
    // Total difficulty (unused in this context)
    totalDifficulty: "0x00",
    // Extra data (unused in this context)
    extraData: "0x",
    // Size (unused in this context)
    size: "0x00",
    // Gas limit
    gasLimit: padString(bigIntToHex(BigInt(DEFAULT_BLOCK_GAS_LIMIT)), 32),
    // Gas used in hexadecimal
    gasUsed: bigIntToHex(BigInt(10)),
    // Timestamp in hexadecimal
    timestamp: "0x64a2c070",
    // Transactions (empty array)
    transactions: [],
    // Uncles (empty array)
    uncles: [],
    // Withdrawals (empty array)
    withdrawals: [],
    // Withdrawals root
    withdrawalsRoot:
      "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    // Base fee per gas (defaulted to zero)
    baseFeePerGas:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
  };

  // Assert that the function's output matches the expected Ethereum header
  assertEquals(
    await toEthHeader({
      header,
      blockNumber,
      blockHash,
      gasUsed,
      logsBloom,
      receiptRoot,
      transactionRoot,
      isPendingBlock,
    }),
    expectedEthHeader,
  );
});

Deno.test("toEthHeader with pending block", async () => {
  // Define a BlockHeader object with necessary properties
  const header: BlockHeader = {
    // Hash of the current block
    blockHash:
      "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df",
    // Hash of the parent block
    parentBlockHash:
      "0x14e330b38856cb7e21b04db7e9fd6872840333ecf753050fe421dfb42b3a3486",
    // Block number in hexadecimal
    blockNumber: "0x1",
    // Address of the sequencer
    sequencerAddress: "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97",
    // New state root
    newRoot:
      "0x3485c8e91ae6107202df98042de861c3229618b46a4bd03407edd84bafeb452f",
    // Timestamp in ISO format
    timestamp: "2023-07-03T12:34:56Z",
  };

  // Block number in hexadecimal
  const blockNumber: PrefixedHexString = "0x1";
  // Hash of the block
  const blockHash: PrefixedHexString =
    "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df";
  // Amount of gas used
  const gasUsed: bigint = BigInt(10);
  // Logs bloom filter
  const logsBloom: Bloom = new Bloom(new Uint8Array(256));
  // Receipts root
  const receiptRoot: Uint8Array = new Uint8Array(32);
  // Transactions root
  const transactionRoot: Uint8Array = new Uint8Array(33);
  // Indicate whether the block is pending
  const isPendingBlock: boolean = true;

  // Define the expected Ethereum header object
  const expectedEthHeader: JsonRpcBlock = {
    // Block number
    number: blockNumber,
    // Block hash (null for pending block)
    hash: NULL_HASH,
    // Parent block hash
    parentHash: header.parentBlockHash,
    // Mix hash (unused in this context)
    mixHash:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    // Nonce (unused in this context)
    nonce: "0x0000000000000000",
    // SHA3 of uncles
    sha3Uncles:
      "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    // Logs bloom in hexadecimal
    logsBloom: bytesToHex(logsBloom.bitvector),
    // Transactions root in hexadecimal
    transactionsRoot: bytesToHex(transactionRoot),
    // New state root
    stateRoot: header.newRoot,
    // Receipts root in hexadecimal
    receiptsRoot: bytesToHex(receiptRoot),
    // Miner address (defaulted to zero address)
    miner: "0x0000000000000000000000000000000000000000",
    // Difficulty (unused in this context)
    difficulty: "0x00",
    // Total difficulty (unused in this context)
    totalDifficulty: "0x00",
    // Extra data (unused in this context)
    extraData: "0x",
    // Size (unused in this context)
    size: "0x00",
    // Gas limit
    gasLimit: padString(bigIntToHex(BigInt(DEFAULT_BLOCK_GAS_LIMIT)), 32),
    // Gas used in hexadecimal
    gasUsed: bigIntToHex(BigInt(10)),
    // Timestamp in hexadecimal
    timestamp: "0x64a2c070",
    // Transactions (empty array)
    transactions: [],
    // Uncles (empty array)
    uncles: [],
    // Withdrawals (empty array)
    withdrawals: [],
    // Root hash of an empty trie
    withdrawalsRoot:
      "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    // Base fee per gas (defaulted to zero)
    baseFeePerGas:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
  };

  // Assert that the function's output matches the expected Ethereum header
  assertEquals(
    await toEthHeader({
      header,
      blockNumber,
      blockHash,
      gasUsed,
      logsBloom,
      receiptRoot,
      transactionRoot,
      isPendingBlock,
    }),
    expectedEthHeader,
  );
});

Deno.test("toEthHeader with invalid timestamp", async () => {
  // Define a BlockHeader object with necessary properties
  const header: BlockHeader = {
    // Hash of the current block
    blockHash:
      "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df",
    // Hash of the parent block
    parentBlockHash:
      "0x14e330b38856cb7e21b04db7e9fd6872840333ecf753050fe421dfb42b3a3486",
    // Block number in hexadecimal
    blockNumber: "0x1",
    // Address of the sequencer
    sequencerAddress: "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97",
    // New state root
    newRoot:
      "0x3485c8e91ae6107202df98042de861c3229618b46a4bd03407edd84bafeb452f",
    // Invalid timestamp format
    timestamp: "Invalid_timestamp",
  };

  // Block number in hexadecimal
  const blockNumber: PrefixedHexString = "0x1";
  // Hash of the block
  const blockHash: PrefixedHexString =
    "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df";
  // Amount of gas used
  const gasUsed: bigint = BigInt(10);
  // Logs bloom filter
  const logsBloom: Bloom = new Bloom(new Uint8Array(256));
  // Receipts root
  const receiptRoot: Uint8Array = new Uint8Array(32);
  // Transactions root
  const transactionRoot: Uint8Array = new Uint8Array(33);
  // Indicate whether the block is pending
  const isPendingBlock: boolean = false;

  // Define the expected Ethereum header object
  const expectedEthHeader: JsonRpcBlock = {
    // Block number
    number: blockNumber,
    // Block hash
    hash: blockHash,
    // Parent block hash
    parentHash: header.parentBlockHash,
    // Mix hash (unused in this context)
    mixHash:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    // Nonce (unused in this context)
    nonce: "0x0000000000000000",
    // SHA3 of uncles
    sha3Uncles:
      "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    // Logs bloom in hexadecimal
    logsBloom: bytesToHex(logsBloom.bitvector),
    // Transactions root in hexadecimal
    transactionsRoot: bytesToHex(transactionRoot),
    // New state root
    stateRoot: header.newRoot,
    // Receipts root in hexadecimal
    receiptsRoot: bytesToHex(receiptRoot),
    // Miner address (defaulted to zero address)
    miner: "0x0000000000000000000000000000000000000000",
    // Difficulty (unused in this context)
    difficulty: "0x00",
    // Total difficulty (unused in this context)
    totalDifficulty: "0x00",
    // Extra data (unused in this context)
    extraData: "0x",
    // Size (unused in this context)
    size: "0x00",
    // Gas limit
    gasLimit: padString(bigIntToHex(BigInt(DEFAULT_BLOCK_GAS_LIMIT)), 32),
    // Gas used in hexadecimal
    gasUsed: bigIntToHex(BigInt(10)),
    // Invalid timestamp should be set to 0
    timestamp: "0x0",
    // Transactions (empty array)
    transactions: [],
    // Uncles (empty array)
    uncles: [],
    // Withdrawals (empty array)
    withdrawals: [],
    // Root hash of an empty trie
    withdrawalsRoot:
      "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    // Base fee per gas (defaulted to zero)
    baseFeePerGas:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
  };

  // Assert that the function's output matches the expected Ethereum header
  assertEquals(
    await toEthHeader({
      header,
      blockNumber,
      blockHash,
      gasUsed,
      logsBloom,
      receiptRoot,
      transactionRoot,
      isPendingBlock,
    }),
    expectedEthHeader,
  );
});

Deno.test("toEthHeader with mocked call", async () => {
  // Define a complete BlockHeader object with necessary properties
  const header: BlockHeader = {
    // Hash of the current block
    blockHash:
      "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df",
    // Hash of the parent block
    parentBlockHash:
      "0x14e330b38856cb7e21b04db7e9fd6872840333ecf753050fe421dfb42b3a3486",
    // Block number in hexadecimal
    blockNumber: "0x1",
    // Address of the sequencer
    sequencerAddress: "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97",
    // New state root
    newRoot:
      "0x3485c8e91ae6107202df98042de861c3229618b46a4bd03407edd84bafeb452f",
    // Timestamp in ISO format
    timestamp: "2023-07-03T12:34:56Z",
  };

  // Block number in hexadecimal
  const blockNumber: PrefixedHexString = "0x1";
  // Hash of the block
  const blockHash: PrefixedHexString =
    "0xf970e4eda704dc2c32a7bf81399716ffff03541ee29aa38b0d8051c2fd2f52df";
  // Amount of gas used
  const gasUsed: bigint = BigInt(10);
  // Logs bloom filter
  const logsBloom: Bloom = new Bloom(new Uint8Array(256));
  // Receipts root
  const receiptRoot: Uint8Array = new Uint8Array(32);
  // Transactions root
  const transactionRoot: Uint8Array = new Uint8Array(33);
  // Indicate whether the block is pending
  const isPendingBlock: boolean = false;

  // Mock the KAKAROT.call function
  const callStub = sinon.stub(KAKAROT, "call").resolves({
    coinbase: BigInt(2),
    base_fee: BigInt(3),
    block_gas_limit: BigInt(4),
  });

  // Define the expected Ethereum header object
  const expectedEthHeader: JsonRpcBlock = {
    // Block number
    number: blockNumber,
    // Block hash
    hash: blockHash,
    // Parent block hash
    parentHash: header.parentBlockHash,
    // Mix hash (unused in this context)
    mixHash:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    // Nonce (unused in this context)
    nonce: "0x0000000000000000",
    // SHA3 of uncles
    sha3Uncles:
      "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    // Logs bloom in hexadecimal
    logsBloom: bytesToHex(logsBloom.bitvector),
    // Transactions root in hexadecimal
    transactionsRoot: bytesToHex(transactionRoot),
    // New state root
    stateRoot: header.newRoot,
    // Receipts root in hexadecimal
    receiptsRoot: bytesToHex(receiptRoot),
    // Miner address
    // Mocked coinbase value
    miner: "0x0000000000000000000000000000000000000002",
    // Difficulty (unused in this context)
    difficulty: "0x00",
    // Total difficulty (unused in this context)
    totalDifficulty: "0x00",
    // Extra data (unused in this context)
    extraData: "0x",
    // Size (unused in this context)
    size: "0x00",
    // Gas limit
    // Mocked block_gas_limit value
    gasLimit:
      "0x0000000000000000000000000000000000000000000000000000000000000004",
    // Gas used in hexadecimal
    gasUsed: bigIntToHex(BigInt(10)),
    // Timestamp in hexadecimal
    timestamp: "0x64a2c070",
    // Transactions (empty array)
    transactions: [],
    // Uncles (empty array)
    uncles: [],
    // Withdrawals (empty array)
    withdrawals: [],
    // Withdrawals root
    withdrawalsRoot:
      "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    // Base fee per gas
    // Mocked base_fee value
    baseFeePerGas:
      "0x0000000000000000000000000000000000000000000000000000000000000003",
  };

  // Assert that the function's output matches the expected Ethereum header
  assertEquals(
    await toEthHeader({
      header,
      blockNumber,
      blockHash,
      gasUsed,
      logsBloom,
      receiptRoot,
      transactionRoot,
      isPendingBlock,
    }),
    expectedEthHeader,
  );

  // Restore the original KAKAROT.call function
  callStub.restore();
});
