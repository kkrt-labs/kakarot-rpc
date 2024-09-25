import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.213.0/assert/mod.ts";
import transform from "./main.ts";
import { toEthHeader } from "./types/header.ts";
import { Bloom } from "./deps.ts";
import { padString, toHexString } from "./utils/hex.ts";
import {
  toTypedEthTx,
  typedTransactionToEthTx,
} from "../src/types/transaction.ts";
import { JsonRpcLog, toEthLog } from "./types/log.ts";
import { toEthReceipt } from "./types/receipt.ts";
import {
  BlockHeader,
  Event,
  EventWithTransaction,
  JsonRpcBlock,
  JsonRpcTx,
  Transaction,
  TransactionReceipt,
  TransactionWithReceipt,
  TypedTransaction,
} from "./deps.ts";
import { JsonRpcReceipt } from "./types/receipt.ts";
import {
  EXPECTED_TRANSFORM_DATA_FILE,
  TRANSACTIONS_DATA_FILE,
} from "./testConstants.ts";

// Transaction data including headers, events, and transactions
const jsonTransactionsData = await Deno.readTextFile(
  TRANSACTIONS_DATA_FILE,
);
const transactionsData = JSON.parse(jsonTransactionsData);

// Expected output after transform and toTypedEthTx transformation for comparison in tests
const jsonExpectedTransformData = await Deno.readTextFile(
  EXPECTED_TRANSFORM_DATA_FILE,
);
const expectedTransformData = JSON.parse(jsonExpectedTransformData);

function assertHasHeader(data: any): asserts data is { header: JsonRpcBlock } {
  if (!data || typeof data.header === "undefined") {
    throw new Error("Expected header not found in result data");
  }
}

function assertHasReceipt(
  data: any,
): asserts data is { receipt: JsonRpcReceipt } {
  if (!data || typeof data.receipt === "undefined") {
    throw new Error("Expected receipt not found in result data");
  }
}

function assertHasTx(data: any): asserts data is { tx: JsonRpcTx } {
  if (!data || typeof data.tx === "undefined") {
    throw new Error("Expected tx not found in result data");
  }
}

function assertHasLog(data: any): asserts data is { log: JsonRpcLog } {
  if (!data || typeof data.log === "undefined") {
    throw new Error("Expected log not found in result data");
  }
}

async function transformData(
  header: BlockHeader,
  events: EventWithTransaction[],
  transactions: TransactionWithReceipt[],
  transaction: Transaction,
  receipt: TransactionReceipt,
  event: Event,
) {
  const result = await transform({ header, events, transactions });

  const expectedHeader = await toEthHeader({
    header: header,
    gasUsed: BigInt(0),
    logsBloom: new Bloom(new Uint8Array(256)),
    receiptRoot: new Uint8Array(),
    transactionRoot: new Uint8Array(),
    blockNumber: padString(toHexString(header.blockNumber), 8),
    blockHash: header.blockHash,
    isPendingBlock: false,
  });
  const typedEthTx = toTypedEthTx({ transaction });
  const ethTx = typedTransactionToEthTx({
    typedTransaction: typedEthTx as TypedTransaction,
    receipt,
    blockNumber: padString(toHexString(header.blockNumber), 8),
    blockHash: header.blockHash,
    isPendingBlock: false,
  });

  const ethLogs = receipt.events
    .map((e: Event) => {
      return toEthLog({
        transaction: ethTx as JsonRpcTx,
        event: e,
        blockNumber: padString(toHexString(header.blockNumber), 8),
        blockHash: header.blockHash,
        isPendingBlock: false,
      });
    })
    .filter((e: JsonRpcLog | null) => e !== null) as JsonRpcLog[];

  const ethLogsIndexed = ethLogs.map((log, index) => {
    log.logIndex = index.toString();
    return log;
  });

  const ethReceipt = toEthReceipt({
    transaction: ethTx as JsonRpcTx,
    logs: ethLogsIndexed,
    event,
    cumulativeGasUsed: 0n,
    blockNumber: padString(toHexString(header.blockNumber), 8),
    blockHash: header.blockHash,
    isPendingBlock: false,
  });

  return { result, expectedHeader, ethTx, ethLogs, ethReceipt };
}

const mockHeader: BlockHeader = {
  blockNumber: "2000",
  blockHash:
    "0x0286731b9083ab0be4a875726f3da0c3c6be2a909f980bd6dccdabe75dde18f5",
  parentBlockHash:
    "0x729fefc61158e655d04e50ca6d1df25970a258cf3a88cfad5579a44907d52a2",
  newRoot: "0x20882c140e56616f37764e40afe2961894e6146777f0a01a565ac06bb04b703",
  timestamp: "1717093503",
  sequencerAddress: "0x1",
};
const mockTransaction: Transaction = {
  meta: {
    hash: "0x361b1ace0f195cb103f8e60c71b168ddfcd4f2bb0d547a1070b473b49ada8b",
    maxFee: "0x7529e57561d443a1",
    signature: [
      "0x8087d6852470e5abf76f858586a929f8",
      "0xb0c2e48b69e902659a47b88abeaca307",
      "0x4bfa2739ec08f51676d7e4a4389bbbd7",
      "0x52a3e8711e1ca2abfb67e7a18cebecb4",
      "0xd6d6e50c",
    ],
    nonce: "0x10a",
    version: "0x1",
  },
  invokeV1: {
    senderAddress:
      "0x4f57d7d15274d9fe832c88aafd80ecc90a8db5696913ddc39cf24af4eba3538",
    calldata: [
      "0x1",
      "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
      "0x7099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
      "0x0",
      "0x4c",
      "0x4c",
      "0xf8",
      "0x4a",
      "0x82",
      "0x1",
      "0xa",
      "0x80",
      "0x83",
      "0x1",
      "0x76",
      "0xb5",
      "0x94",
      "0x16",
      "0x35",
      "0xbf",
      "0x43",
      "0x5f",
      "0x73",
      "0x62",
      "0x8d",
      "0x86",
      "0x20",
      "0x35",
      "0x69",
      "0x55",
      "0xf4",
      "0x79",
      "0xfc",
      "0x54",
      "0xff",
      "0xd3",
      "0xdd",
      "0x80",
      "0xa4",
      "0x75",
      "0x5e",
      "0xdd",
      "0x17",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x4d",
      "0x6a",
      "0xa6",
      "0xa7",
      "0x29",
      "0xc2",
      "0x8",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x0",
      "0x84",
      "0x6b",
      "0x6b",
      "0x72",
      "0x74",
      "0x80",
      "0x80",
    ],
  },
};
const mockReceipt: TransactionReceipt = {
  executionStatus: "EXECUTION_STATUS_SUCCEEDED",
  transactionHash:
    "0x361b1ace0f195cb103f8e60c71b168ddfcd4f2bb0d547a1070b473b49ada8b",
  transactionIndex: "1",
  actualFee: "0x19452efbea3d",
  contractAddress: "0xcontract1",
  l2ToL1Messages: [],
  events: [
    {
      fromAddress:
        "0xe4c697374a19d04f21ed16f4755f75328f508b0f7515bc929dc05b65116207",
      keys: [
        "0x1390fd803c110ac71730ece1decfc34eb1d0088e295d4f1b125dda1e0c5b9ff",
      ],
      data: [
        "0x0",
        "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
      ],
      index: 0,
    },
    {
      fromAddress:
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
      keys: [
        "0x134692b230b9e1ffa39098904722134159652b09c5bc41d88d6698779d228ff",
      ],
      data: [
        "0xe4c697374a19d04f21ed16f4755f75328f508b0f7515bc929dc05b65116207",
        "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
        "0xffffffffffffffffffffffffffffffff",
        "0xffffffffffffffffffffffffffffffff",
      ],
      index: 1,
    },
    {
      fromAddress:
        "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
      keys: [
        "0xf85617d642704f0a8a5647db56a1492a44de95131dff7326e9349e6362a2c",
      ],
      data: [
        "0x4d6aa6a729c20800000000000000000000000000",
        "0xe4c697374a19d04f21ed16f4755f75328f508b0f7515bc929dc05b65116207",
      ],
      index: 2,
    },
    {
      fromAddress:
        "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
      keys: [
        "0x1635bf435f73628d8620356955f479fc54ffd3dd",
        "0x952ba7f163c4a11628f55a4df523b3ef",
        "0xddf252ad1be2c89b69c2b068fc378daa",
        "0x0",
        "0x0",
        "0x29c20800000000000000000000000000",
        "0x4d6aa6a7",
        "0x7f",
        "0x0",
      ],
      data: [],
      index: 3,
    },
    {
      fromAddress:
        "0x4f57d7d15274d9fe832c88aafd80ecc90a8db5696913ddc39cf24af4eba3538",
      keys: [
        "0x5ad857f66a5b55f1301ff1ed7e098ac6d4433148f0b72ebc4a2945ab85ad53",
      ],
      data: [
        "0x20",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x0",
        "0x7f",
        "0x1",
        "0x13842",
      ],
      index: 4,
    },
    {
      fromAddress:
        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
      keys: [
        "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9",
      ],
      data: [
        "0x4f57d7d15274d9fe832c88aafd80ecc90a8db5696913ddc39cf24af4eba3538",
        "0x46a89ae102987331d369645031b49c27738ed096f2789c24449966da4c6de6b",
        "0x19452efbea3d",
        "0x0",
      ],
      index: 5,
    },
  ],
};
const mockEvent: Event = {
  fromAddress:
    "0x4f57d7d15274d9fe832c88aafd80ecc90a8db5696913ddc39cf24af4eba3538",
  keys: [
    "0x5ad857f66a5b55f1301ff1ed7e098ac6d4433148f0b72ebc4a2945ab85ad53",
  ],
  data: [
    "0x20",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x7f",
    "0x1",
    "0x13842",
  ],
  index: 4,
};
const mockEvents: EventWithTransaction[] = [
  {
    transaction: mockTransaction,
    receipt: mockReceipt,
    event: mockEvent,
  },
];
const mockTransactions: TransactionWithReceipt[] = [
  {
    transaction: mockTransaction,
    receipt: mockReceipt,
  },
];

Deno.test("transform with no events or transactions", async () => {
  const result = await transform({
    header: mockHeader,
    events: [],
    transactions: [],
  });

  const expectedHeader = await toEthHeader({
    header: mockHeader,
    gasUsed: BigInt(0),
    logsBloom: new Bloom(new Uint8Array(256)),
    receiptRoot: new Uint8Array(),
    transactionRoot: new Uint8Array(),
    blockNumber: padString(toHexString(mockHeader.blockNumber), 8),
    blockHash: mockHeader.blockHash,
    isPendingBlock: false,
  });

  assertEquals(result.length, 1);
  assertEquals(result[0].collection, "headers");

  assertHasHeader(result[0].data);
  assertEquals(result[0].data.header.number, expectedHeader.number);
  assertEquals(result[0].data.header.hash, expectedHeader.hash);
  assertEquals(result[0].data.header.parentHash, expectedHeader.parentHash);
  assertEquals(result[0].data.header.sha3Uncles, expectedHeader.sha3Uncles);
  assertEquals(result[0].data.header.stateRoot, expectedHeader.stateRoot);
  assertEquals(result[0].data.header.gasLimit, expectedHeader.gasLimit);
  assertEquals(
    result[0].data.header.withdrawalsRoot,
    expectedHeader.withdrawalsRoot,
  );
  assertEquals(result[0].data.header.transactions.length, 0);
});

Deno.test.ignore("transform without logs", async () => {
  const receiptWithoutLog: TransactionReceipt = {
    executionStatus: "EXECUTION_STATUS_SUCCEEDED",
    transactionHash:
      "0x160d046afe08256267fa76b66e97f6a553ffc80c09cb5dc0ab0fc5d28c05658",
    transactionIndex: "0",
    actualFee: "0x19452efbea3d",
    contractAddress: "0xcontract1",
    l2ToL1Messages: [],
    events: [
      {
        fromAddress:
          "0x1daabdc9a68ac6094ae282891913ac2470424809556564445182a4207f77355",
        keys: [
          "0x1390fd803c110ac71730ece1decfc34eb1d0088e295d4f1b125dda1e0c5b9ff",
        ],
        data: [
          "0x0",
          "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
        ],
        index: 0,
      },
      {
        fromAddress:
          "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        keys: [
          "0x134692b230b9e1ffa39098904722134159652b09c5bc41d88d6698779d228ff",
        ],
        data: [
          "0x1daabdc9a68ac6094ae282891913ac2470424809556564445182a4207f77355",
          "0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda",
          "0xffffffffffffffffffffffffffffffff",
          "0xffffffffffffffffffffffffffffffff",
        ],
        index: 1,
      },
      {
        fromAddress:
          "0x4f57d7d15274d9fe832c88aafd80ecc90a8db5696913ddc39cf24af4eba3538",
        keys: [
          "0x5ad857f66a5b55f1301ff1ed7e098ac6d4433148f0b72ebc4a2945ab85ad53",
        ],
        data: [
          "0x20",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x0",
          "0x7e",
          "0x1",
          "0x13842",
        ],
        index: 0,
      },
    ],
  };
  const mockEvents: EventWithTransaction[] = [
    {
      transaction: mockTransaction,
      receipt: receiptWithoutLog,
      event: mockEvent,
    },
  ];
  const mockTransactions: TransactionWithReceipt[] = [
    {
      transaction: mockTransaction,
      receipt: receiptWithoutLog,
    },
  ];

  const { result, expectedHeader, ethTx, ethLogs, ethReceipt } =
    await transformData(
      mockHeader,
      mockEvents,
      mockTransactions,
      mockTransaction,
      receiptWithoutLog,
      mockEvent,
    );

  assertEquals(result.length, 3);

  assertHasTx(result[0].data);
  assertEquals(result[0].data.tx, ethTx);

  assertHasReceipt(result[1].data);
  assertEquals(result[1].data.receipt, ethReceipt);

  assertHasHeader(result[2].data);
  assertEquals(result[2].data.header.number, expectedHeader.number);
  assertEquals(result[2].data.header.hash, expectedHeader.hash);
  assertEquals(result[2].data.header.parentHash, expectedHeader.parentHash);
  assertEquals(result[2].data.header.sha3Uncles, expectedHeader.sha3Uncles);
  assertEquals(result[2].data.header.stateRoot, expectedHeader.stateRoot);
  assertEquals(result[2].data.header.gasLimit, expectedHeader.gasLimit);
  assertEquals(
    result[2].data.header.withdrawalsRoot,
    expectedHeader.withdrawalsRoot,
  );
  assertEquals(result[2].data.header.transactions.length, 0);
});

Deno.test.ignore("transform with logs events and transaction", async () => {
  const { result, expectedHeader, ethTx, ethLogs, ethReceipt } =
    await transformData(
      mockHeader,
      mockEvents,
      mockTransactions,
      mockTransaction,
      mockReceipt,
      mockEvent,
    );

  assertEquals(result.length, 4);

  assertHasTx(result[0].data);
  assertEquals(result[0].data.tx, ethTx);

  assertHasReceipt(result[1].data);
  assertEquals(result[1].data.receipt, ethReceipt);

  assertHasLog(result[2].data);
  assertEquals(result[2].data.log, ethLogs[0]);

  assertHasHeader(result[3].data);
  assertEquals(result[3].data.header.number, expectedHeader.number);
  assertEquals(result[3].data.header.hash, expectedHeader.hash);
  assertEquals(result[3].data.header.parentHash, expectedHeader.parentHash);
  assertEquals(result[3].data.header.sha3Uncles, expectedHeader.sha3Uncles);
  assertEquals(result[3].data.header.stateRoot, expectedHeader.stateRoot);
  assertEquals(result[3].data.header.gasLimit, expectedHeader.gasLimit);
  assertEquals(
    result[3].data.header.withdrawalsRoot,
    expectedHeader.withdrawalsRoot,
  );
  assertEquals(result[3].data.header.transactions.length, 0);
});

Deno.test.ignore("transform with real data", async () => {
  const { headersList, eventsList, transactionsList } = transactionsData;

  for (let i = 0; i < headersList.length; i++) {
    const header = headersList[i];
    const events = eventsList[i];
    const transactions = transactionsList[i];
    const expectedTransformedData = expectedTransformData.expectedTransform[i];

    const result = await transform({
      header: header,
      events: events,
      transactions: transactions,
    });

    assertEquals(
      JSON.stringify(result),
      JSON.stringify(expectedTransformedData),
    );
  }
});
