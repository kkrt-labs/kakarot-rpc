import {
  assert,
  assertFalse,
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
  assert(ethValidationFailed(x));
});

Deno.test(
  "ethValidationFailed: wrong failure message should not be taken into account",
  () => {
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
      "0x65", // Modify this randomly to change the output message
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
    assertFalse(ethValidationFailed(x));
  },
);

Deno.test("ethValidationFailed: empty data", () => {
  const x = event([]);
  assertFalse(ethValidationFailed(x));
});

Deno.test("ethValidationFailed: success true", () => {
  const x = event(["0x1", "0x1", "0x1", "0x1"]);
  assertFalse(ethValidationFailed(x));
});

Deno.test("ethValidationFailed: incorrect data length", () => {
  const x = event(["0x10"]);
  assertFalse(ethValidationFailed(x));
});

Deno.test(
  "isKakarotTransaction: InvokeTransactionV0 should not be indexed",
  () => {
    const starknetTxCalldata: `0x${string}`[] = ["0x1", "0x1"];
    const transaction: Transaction = {
      invokeV0: {
        contractAddress: "0x01",
        entryPointSelector: "0x01",
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
    assertFalse(isKakarotTransaction(transaction));
  },
);

Deno.test(
  "isKakarotTransaction: L1HandlerTransaction should not be indexed",
  () => {
    const starknetTxCalldata: `0x${string}`[] = ["0x1", "0x1"];
    const transaction: Transaction = {
      l1Handler: {
        contractAddress: "0x01",
        entryPointSelector: "0x01",
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
    assertFalse(isKakarotTransaction(transaction));
  },
);

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
  assertFalse(isKakarotTransaction(transaction));
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
  assertFalse(isKakarotTransaction(transaction));
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
    assertFalse(isKakarotTransaction(transaction));
  },
);

Deno.test.ignore(
  "isKakarotTransaction: `to` address matching KAKAROT_ADDRESS",
  () => {
    const starknetTxCalldata: `0x${string}`[] = [
      "0x1",
      "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
    ];
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
    assert(isKakarotTransaction(transaction));
  },
);

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
      revertReason:
        "Could not reach the end of the program. RunResources has no remaining steps",
    };
    assert(isRevertedWithOutOfResources(receipt));
  },
);

Deno.test(
  "isRevertedWithOutOfResources: false on status reverted and no revert reason",
  () => {
    const receipt: TransactionReceipt = {
      executionStatus: "EXECUTION_STATUS_REVERTED",
      transactionHash: "0x01",
      transactionIndex: "0x01",
      actualFee: "0x01",
      contractAddress: "0x01",
      l2ToL1Messages: [],
      events: [],
    };
    assertFalse(isRevertedWithOutOfResources(receipt));
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
    revertReason:
      "Could not reach the end of the program. RunResources has no remaining steps",
  };
  assertFalse(isRevertedWithOutOfResources(receipt));
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
    assertFalse(isRevertedWithOutOfResources(receipt));
  },
);
