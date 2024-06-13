import {
  assert,
  assertFalse,
  assertEquals,
} from "https://deno.land/std@0.213.0/assert/mod.ts";
import {
  ethValidationFailed,
  isKakarotTransaction,
  isRevertedWithOutOfResources,
} from "./filter.ts";
import { Event, Transaction, TransactionReceipt } from "../deps.ts";

const event = (data: `0x${string}`[]) => {
  return {
    index: 1,
    fromAddress: "0x1",
    keys: ["0x1", "0x2", "0x3"],
    data,
  } as Event;
};

Deno.test("ethValidationFailed: failed transaction", () => {
  const x = event([
    "0x1e",
    "0x4b",
    "0x61",
    "0x6b",
    "0x61",
    "0x72",
    "0x6f",
    "0x74",
    "0x3a",
    "0x20",
    "0x65",
    "0x74",
    "0x68",
    "0x20",
    "0x76",
    "0x61",
    "0x6c",
    "0x69",
    "0x64",
    "0x61",
    "0x74",
    "0x69",
    "0x6f",
    "0x6e",
    "0x20",
    "0x66",
    "0x61",
    "0x69",
    "0x6c",
    "0x65",
    "0x64",
    "0x0",
    "0x0",
  ]);
  const failed = ethValidationFailed(x);
  assert(failed);
});

Deno.test("ethValidationFailed: empty data", () => {
  const x = event([]);
  const failed = ethValidationFailed(x);
  assertFalse(failed);
});

Deno.test("ethValidationFailed: success true", () => {
  const x = event(["0x1", "0x1", "0x1", "0x1"]);
  const failed = ethValidationFailed(x);
  assertFalse(failed);
});

Deno.test("ethValidationFailed: incorrect data length", () => {
  const x = event(["0x10"]);
  const failed = ethValidationFailed(x);
  assertFalse(failed);
});

Deno.test("isKakarotTransaction: no calldata", () => {
  const transaction: Transaction = {
    invokeV1: {
      senderAddress: "0x01",
      calldata: [],
    },
    meta: {
      hash: "0x01",
      maxFee: "0x01",
      nonce: "0x01",
      signature: ["0x1", "0x2", "0x3", "0x4", "0x32"],
      version: "1",
    },
  };
  const failed = isKakarotTransaction(transaction);
  assertFalse(failed);
});

Deno.test("isKakarotTransaction: no `to` field in calldata", () => {
  const starknetTxCalldata: `0x${string}`[] = ["0x1"];
  const transaction: Transaction = {
    invokeV1: {
      senderAddress: "0x01",
      calldata: starknetTxCalldata,
    },
    meta: {
      hash: "0x01",
      maxFee: "0x01",
      nonce: "0x01",
      signature: ["0x1", "0x2", "0x3", "0x4", "0x32"],
      version: "1",
    },
  };
  const failed = isKakarotTransaction(transaction);
  assertFalse(failed);
});

Deno.test(
  "isKakarotTransaction: `to` address not matching KAKAROT_ADDRESS",
  () => {
    const starknetTxCalldata: `0x${string}`[] = ["0x1", "0x2"];
    const transaction: Transaction = {
      invokeV1: {
        senderAddress: "0x01",
        calldata: starknetTxCalldata,
      },
      meta: {
        hash: "0x02",
        maxFee: "0x02",
        nonce: "0x02",
        signature: ["0x2"],
        version: "1",
      },
    };
    const failed = isKakarotTransaction(transaction);
    assertEquals(failed, false);
  },
);

Deno.test("isKakarotTransaction: `to` address matching KAKAROT_ADDRESS", () => {
  const starknetTxCalldata: `0x${string}`[] = ["0x1", "0x1"];
  const transaction: Transaction = {
    invokeV1: {
      senderAddress: "0x01",
      calldata: starknetTxCalldata,
    },
    meta: {
      hash: "0x01",
      maxFee: "0x01",
      nonce: "0x01",
      signature: ["0x1", "0x2", "0x3", "0x4", "0x1"],
      version: "1",
    },
  };
  const success = isKakarotTransaction(transaction);
  assertEquals(success, true);
});

Deno.test(
  "isRevertedWithOutOfResources: true on status reverted and revert reason",
  () => {
    const receipt: TransactionReceipt = {
      executionStatus: "EXECUTION_STATUS_REVERTED",
      transactionHash: "0x01",
      transactionIndex: "0x01",
      actualFee: "0x01",
      contractAddress: "0x01",
      l2ToL1Messages: [],
      events: [],
      revertReason: "RunResources has no remaining steps",
    };
    const success = isRevertedWithOutOfResources(receipt);
    assertEquals(success, true);
  },
);

Deno.test("isRevertedWithOutOfResources: false on status succeeded", () => {
  const receipt: TransactionReceipt = {
    executionStatus: "EXECUTION_STATUS_SUCCEEDED",
    transactionHash: "0x01",
    transactionIndex: "0x01",
    actualFee: "0x01",
    contractAddress: "0x01",
    l2ToL1Messages: [],
    events: [],
    revertReason: "RunResources has no remaining steps",
  };
  const success = isRevertedWithOutOfResources(receipt);
  assertEquals(success, false);
});

Deno.test(
  "isRevertedWithOutOfResources: false on incorrect revert reason",
  () => {
    const receipt: TransactionReceipt = {
      executionStatus: "EXECUTION_STATUS_REVERTED",
      transactionHash: "0x01",
      transactionIndex: "0x01",
      actualFee: "0x01",
      contractAddress: "0x01",
      l2ToL1Messages: [],
      events: [],
      revertReason: "eth validation failed",
    };
    const success = isRevertedWithOutOfResources(receipt);
    assertEquals(success, false);
  },
);
