import { assertEquals } from "https://deno.land/std@0.213.0/assert/mod.ts";
import { fromJsonRpcLog, toEthLog } from "../src/types/log.ts";
import { JsonRpcLog } from "../src/types/types.ts";
import { bigIntToHex, Event, JsonRpcTx } from "../src/deps.ts";
import { IGNORED_KEYS, KAKAROT_ADDRESS } from "../src/constants.ts";

// Mock for hexToBytes
const mockHexToBytes = (hex: string): Uint8Array => {
  // Remove the '0x' prefix if present
  const cleanedHex = hex.startsWith("0x") ? hex.slice(2) : hex;

  // Match pairs of hex characters
  const hexPairs = cleanedHex.match(/.{1,2}/g) || [];

  // Convert pairs to integers and create a Uint8Array
  return new Uint8Array(hexPairs.map((x) => parseInt(x, 16)));
};

Deno.test("fromJsonRpcLog with valid input", () => {
  const log: JsonRpcLog = {
    removed: false,
    logIndex: "1",
    transactionIndex: "0",
    transactionHash: "0x1234567890abcdef",
    blockHash: "0xabcdef1234567890",
    blockNumber: "10",
    address: "0x1234567890abcdef1234567890abcdef12345678",
    data: "0xabcdef",
    topics: ["0x1234", "0x5678"],
  };

  const expected: [Uint8Array, Uint8Array[], Uint8Array] = [
    mockHexToBytes("0x1234567890abcdef1234567890abcdef12345678"),
    [mockHexToBytes("0x1234"), mockHexToBytes("0x5678")],
    mockHexToBytes("0xabcdef"),
  ];

  assertEquals(fromJsonRpcLog(log), expected);
});

Deno.test("fromJsonRpcLog with empty data", () => {
  const log: JsonRpcLog = {
    removed: false,
    logIndex: null,
    transactionIndex: null,
    transactionHash: null,
    blockHash: null,
    blockNumber: null,
    address: "0x1234",
    data: "0x",
    topics: [],
  };

  const expected: [Uint8Array, Uint8Array[], Uint8Array] = [
    mockHexToBytes("0x1234"),
    [],
    mockHexToBytes("0x"),
  ];

  assertEquals(fromJsonRpcLog(log), expected);
});

Deno.test("fromJsonRpcLog with no topics", () => {
  const log: JsonRpcLog = {
    removed: false,
    logIndex: null,
    transactionIndex: null,
    transactionHash: null,
    blockHash: null,
    blockNumber: null,
    address: "0x1234",
    data: "0x1234",
    topics: [] as string[],
  };

  const expected: [Uint8Array, Uint8Array[], Uint8Array] = [
    mockHexToBytes("0x1234"),
    [],
    mockHexToBytes("0x1234"),
  ];

  assertEquals(fromJsonRpcLog(log), expected);
});

Deno.test("toEthLog with valid input", () => {
  const transaction: JsonRpcTx = {
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    from: "0x1234567890abcdef",
    gas: "0x5208",
    gasPrice: "0x3b9aca00",
    type: "0x2",
    hash: "0xhash",
    input: "0xinput",
    nonce: "0x0",
    to: "0xabcdef",
    transactionIndex: "0x1",
    value: "0xde0b6b3a7640000",
    v: "0x1b",
    r: "0x1c",
    s: "0x1d",
  };

  const event: Event = {
    index: 1,
    fromAddress: KAKAROT_ADDRESS as `0x${string}`,
    keys: ["0x1234", "0x5678", "0x9abc", "0xdef0", "0x1111"],
    data: ["0x01", "0x02", "0x03"],
  };

  const expected = {
    removed: false,
    logIndex: null,
    transactionIndex: "0x1",
    transactionHash: "0xhash",
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    address: "0x0000000000000000000000000000000000001234",
    data: "0x010203",
    topics: [
      "0x00000000000000000000000000009abc00000000000000000000000000005678",
      "0x000000000000000000000000000011110000000000000000000000000000def0",
    ],
  };

  assertEquals(
    toEthLog({
      transaction,
      event,
      blockNumber: "0xa",
      blockHash: "0xabcdef1234567890",
      isPendingBlock: false,
    }),
    expected,
  );
});

Deno.test("toEthLog with invalid event keys length", () => {
  const transaction: JsonRpcTx = {
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    from: "0x1234567890abcdef",
    gas: "0x5208",
    gasPrice: "0x3b9aca00",
    type: "0x2",
    hash: "0xhash",
    input: "0xinput",
    nonce: "0x0",
    to: "0xabcdef",
    transactionIndex: "0x1",
    value: "0xde0b6b3a7640000",
    v: "0x1b",
    r: "0x1c",
    s: "0x1d",
  };

  // The event must have an odd number of keys.
  const event: Event = {
    index: 1,
    fromAddress: "0x123456",
    keys: ["0x1234", "0x5678"],
    data: ["0x01", "0x02", "0x03"],
  };

  assertEquals(
    toEthLog({
      transaction,
      event,
      blockNumber: "0xa",
      blockHash: "0xabcdef1234567890",
      isPendingBlock: false,
    }),
    null,
  );
});

Deno.test("toEthLog with no event key", () => {
  const transaction: JsonRpcTx = {
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    from: "0x1234567890abcdef",
    gas: "0x5208",
    gasPrice: "0x3b9aca00",
    type: "0x2",
    hash: "0xhash",
    input: "0xinput",
    nonce: "0x0",
    to: "0xabcdef",
    transactionIndex: "0x1",
    value: "0xde0b6b3a7640000",
    v: "0x1b",
    r: "0x1c",
    s: "0x1d",
  };

  // The event must have at least one key.
  const event: Event = {
    index: 1,
    fromAddress: "0x123456",
    keys: [],
    data: ["0x01", "0x02", "0x03"],
  };

  assertEquals(
    toEthLog({
      transaction,
      event,
      blockNumber: "0xa",
      blockHash: "0xabcdef1234567890",
      isPendingBlock: false,
    }),
    null,
  );
});

Deno.test("toEthLog with empty event data", () => {
  const transaction: JsonRpcTx = {
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    from: "0x1234567890abcdef",
    gas: "0x5208",
    gasPrice: "0x3b9aca00",
    type: "0x2",
    hash: "0xhash",
    input: "0xinput",
    nonce: "0x0",
    to: "0xabcdef",
    transactionIndex: "0x1",
    value: "0xde0b6b3a7640000",
    v: "0x1b",
    r: "0x1c",
    s: "0x1d",
  };

  // No data in the event.
  const event: Event = {
    index: 1,
    fromAddress: KAKAROT_ADDRESS as `0x${string}`,
    keys: ["0x1234", "0x5678", "0x9abc", "0xdef0", "0x1111"],
    data: [],
  };

  const expected = {
    removed: false,
    logIndex: null,
    transactionIndex: "0x1",
    transactionHash: "0xhash",
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    address: "0x0000000000000000000000000000000000001234",
    data: "0x",
    topics: [
      "0x00000000000000000000000000009abc00000000000000000000000000005678",
      "0x000000000000000000000000000011110000000000000000000000000000def0",
    ],
  };

  assertEquals(
    toEthLog({
      transaction,
      event,
      blockNumber: "0xa",
      blockHash: "0xabcdef1234567890",
      isPendingBlock: false,
    }),
    expected,
  );
});

Deno.test("toEthLog with pending block", () => {
  const transaction: JsonRpcTx = {
    blockHash: null,
    blockNumber: null,
    from: "0x1234567890abcdef",
    gas: "0x5208",
    gasPrice: "0x3b9aca00",
    type: "0x2",
    hash: "0xhash",
    input: "0xinput",
    nonce: "0x0",
    to: "0xabcdef",
    transactionIndex: "0x1",
    value: "0xde0b6b3a7640000",
    v: "0x1b",
    r: "0x1c",
    s: "0x1d",
  };

  const event: Event = {
    index: 1,
    fromAddress: KAKAROT_ADDRESS as `0x${string}`,
    keys: ["0x1234", "0x5678", "0x9abc", "0xdef0", "0x1111"],
    data: ["0x01", "0x02", "0x03"],
  };

  const expected = {
    removed: false,
    logIndex: null,
    transactionIndex: "0x1",
    transactionHash: "0xhash",
    // Null block hash
    blockHash:
      "0x0000000000000000000000000000000000000000000000000000000000000000",
    blockNumber: "0xa",
    address: "0x0000000000000000000000000000000000001234",
    data: "0x010203",
    topics: [
      "0x00000000000000000000000000009abc00000000000000000000000000005678",
      "0x000000000000000000000000000011110000000000000000000000000000def0",
    ],
  };

  assertEquals(
    toEthLog({
      transaction,
      event,
      blockNumber: "0xa",
      blockHash: "0xabcdef1234567890",
      // Pending block
      isPendingBlock: true,
    }),
    expected,
  );
});

Deno.test("toEthLog with ignored keys", () => {
  const transaction: JsonRpcTx = {
    blockHash: "0xabcdef1234567890",
    blockNumber: "0xa",
    from: "0x1234567890abcdef",
    gas: "0x5208",
    gasPrice: "0x3b9aca00",
    type: "0x2",
    hash: "0xhash",
    input: "0xinput",
    nonce: "0x0",
    to: "0xabcdef",
    transactionIndex: "0x1",
    value: "0xde0b6b3a7640000",
    v: "0x1b",
    r: "0x1c",
    s: "0x1d",
  };

  // Loop through all ignored keys.
  for (const ignoredKey of IGNORED_KEYS) {
    const event: Event = {
      index: 1,
      fromAddress: "0x123456",
      keys: [bigIntToHex(ignoredKey) as `0x${string}`],
      data: ["0x01", "0x02", "0x03"],
    };

    assertEquals(
      toEthLog({
        transaction,
        event,
        blockNumber: "0xa",
        blockHash: "0xabcdef1234567890",
        isPendingBlock: false,
      }),
      null,
    );
  }
});
