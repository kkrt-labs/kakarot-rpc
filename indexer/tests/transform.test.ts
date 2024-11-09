import { assertEquals } from "https://deno.land/std@0.213.0/assert/mod.ts";
import transform from "../src/main.ts";
import { toEthHeader } from "../src/types/header.ts";
import { BlockHeader, Bloom, JsonRpcBlock } from "../src/deps.ts";
import { padString, toHexString } from "../src/utils/hex.ts";
import {
  EXPECTED_TRANSFORM_DATA_FILE,
  TRANSACTIONS_DATA_FILE,
} from "./constants.ts";

// Transaction data including headers, events, and transactions
const jsonTransactionsData = await Deno.readTextFile(TRANSACTIONS_DATA_FILE);
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

Deno.test("transform with real data", async () => {
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

    // Remove the miner and baseFeePerGas from the header
    // This is done because the test doesn't have access to
    // the Starknet network and hence won't be able to query
    // the correct values
    for (let j = 0; j < expectedTransformedData.length; j++) {
      if (expectedTransformedData[j].collection === "headers") {
        const value = expectedTransformedData[j].data as {
          header: JsonRpcBlock;
        };
        value.header.miner = "0x0000000000000000000000000000000000000000";
        value.header.baseFeePerGas =
          "0x0000000000000000000000000000000000000000000000000000000000000000";
        expectedTransformedData[j].data = value;
      }
    }

    assertEquals(
      JSON.stringify(result),
      JSON.stringify(expectedTransformedData),
    );
  }
});
