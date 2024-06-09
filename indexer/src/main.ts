// Utils
import { NULL_BLOCK_HASH, padString, toHexString } from "./utils/hex.ts";
import { ethValidationFailed, isKakarotTransaction } from "./utils/filter.ts";

// Types
import { toEthTx, toTypedEthTx } from "./types/transaction.ts";
import { toEthHeader } from "./types/header.ts";
import { fromJsonRpcReceipt, toEthReceipt } from "./types/receipt.ts";
import { JsonRpcLog, toEthLog } from "./types/log.ts";
import { StoreItem } from "./types/storeItem.ts";
// Starknet
import {
  BlockHeader,
  Config,
  EventWithTransaction,
  hash,
  NetworkOptions,
  SinkOptions,
} from "./deps.ts";
// Eth
import { Bloom, encodeReceipt, hexToBytes, RLP, Trie } from "./deps.ts";

const AUTH_TOKEN = Deno.env.get("APIBARA_AUTH_TOKEN") ?? "";
const TRANSACTION_EXECUTED = hash.getSelectorFromName("transaction_executed");

const STREAM_URL = Deno.env.get("STREAM_URL") ?? "http://localhost:7171";
const STARTING_BLOCK = Number(Deno.env.get("STARTING_BLOCK")) ?? 0;
if (!Number.isSafeInteger(STARTING_BLOCK) || STARTING_BLOCK < 0) {
  throw new Error("Invalid STARTING_BLOCK");
}
const SINK_TYPE = Deno.env.get("SINK_TYPE") ?? "console";
if (SINK_TYPE !== "console" && SINK_TYPE !== "mongo") {
  throw new Error("Invalid SINK_TYPE");
}

const sinkOptions = SINK_TYPE === "mongo"
  ? {
    connectionString: Deno.env.get("MONGO_CONNECTION_STRING") ??
      "mongodb://mongo:mongo@mongo:27017",
    database: Deno.env.get("MONGO_DATABASE_NAME") ?? "kakarot-test-db",
    collectionNames: ["headers", "transactions", "receipts", "logs"],
  }
  : {};

export const config: Config<NetworkOptions, SinkOptions> = {
  streamUrl: STREAM_URL,
  authToken: AUTH_TOKEN,
  startingBlock: STARTING_BLOCK,
  network: "starknet",
  finality: "DATA_STATUS_PENDING",
  filter: {
    header: { weak: false },
    // Filters are unions
    events: [
      {
        keys: [TRANSACTION_EXECUTED],
      },
    ],
  },
  sinkType: SINK_TYPE,
  sinkOptions: sinkOptions,
};

export default async function transform({
  header,
  events,
}: {
  header: BlockHeader;
  events: EventWithTransaction[];
}) {
  // Accumulate the gas used in the block in order to calculate the cumulative gas used.
  // We increment it by the gas used in each transaction in the flatMap iteration.
  let cumulativeGasUsed = 0n;
  const blockNumber = padString(toHexString(header.blockNumber), 8);
  const isPendingBlock = padString(header.blockHash, 32) === NULL_BLOCK_HASH;
  const blockHash = padString(header.blockHash, 32);
  const blockLogsBloom = new Bloom();
  const transactionTrie = new Trie();
  const receiptTrie = new Trie();

  const store: Array<StoreItem> = [];

  await Promise.all(
    (events ?? []).map(async ({ transaction, receipt, event }) => {
      // Can be false if the transaction is not related to a specific instance of the Kakarot contract.
      // This is typically the case if there are multiple Kakarot contracts on the same chain.
      console.log(
        "🔍 Processing transaction with Starknet hash: ",
        transaction.meta.hash,
      );
      const isKakarotTx = isKakarotTransaction(transaction);
      if (!isKakarotTx) {
        return null;
      }

      // Skip if the transaction_executed event contains "eth validation failed".
      if (ethValidationFailed(event)) {
        return null;
      }

      const typedEthTx = toTypedEthTx({ transaction });
      // Can be null if:
      // 1. The transaction is missing calldata.
      // 2. The transaction is a multi-call.
      // 3. The length of the signature array is different from 5.
      // 4. The chain id is not encoded in the v param of the signature for a
      //    Legacy transaction.
      // 5. The deserialization of the transaction fails.
      if (typedEthTx === null) {
        return null;
      }
      const ethTx = toEthTx({
        transaction: typedEthTx,
        receipt,
        blockNumber,
        blockHash,
        isPendingBlock,
      });
      // Can be null if:
      // 1. The typed transaction if missing a signature param (v, r, s).
      if (ethTx === null) {
        return null;
      }

      // Can be null if:
      // 1. The event is part of the defined ignored events (see IGNORED_KEYS).
      // 2. The event has an invalid number of keys.
      const ethLogs = receipt.events
        .map((e) => {
          return toEthLog({
            transaction: ethTx,
            event: e,
            blockNumber,
            blockHash,
            isPendingBlock,
          });
        })
        .filter((e) => e !== null) as JsonRpcLog[];
      const ethLogsIndexed = ethLogs.map((log, index) => {
        log.logIndex = index.toString();
        return log;
      });

      const ethReceipt = toEthReceipt({
        transaction: ethTx,
        logs: ethLogsIndexed,
        event,
        cumulativeGasUsed,
        blockNumber,
        blockHash,
        isPendingBlock,
      });

      // Trie code is based off:
      // - https://github.com/ethereumjs/ethereumjs-monorepo/blob/master/packages/block/src/block.ts#L85
      // - https://github.com/ethereumjs/ethereumjs-monorepo/blob/master/packages/vm/src/buildBlock.ts#L153
      // Add the transaction to the transaction trie.
      await transactionTrie.put(
        RLP.encode(Number(ethTx.transactionIndex)),
        typedEthTx.serialize(),
      );
      // Add the receipt to the receipt trie.
      const encodedReceipt = encodeReceipt(
        fromJsonRpcReceipt(ethReceipt),
        typedEthTx.type,
      );
      await receiptTrie.put(
        RLP.encode(Number(ethTx.transactionIndex)),
        encodedReceipt,
      );
      // Add the logs bloom of the receipt to the block logs bloom.
      const receiptBloom = new Bloom(hexToBytes(ethReceipt.logsBloom));
      blockLogsBloom.or(receiptBloom);
      cumulativeGasUsed += BigInt(ethReceipt.gasUsed);

      // Add all the eth data to the store.
      store.push({ collection: "transactions", data: { tx: ethTx } });
      store.push({ collection: "receipts", data: { receipt: ethReceipt } });
      ethLogs.forEach((ethLog) => {
        store.push({ collection: "logs", data: { log: ethLog } });
      });
    }),
  );

  const ethHeader = await toEthHeader({
    header: header,
    gasUsed: cumulativeGasUsed,
    logsBloom: blockLogsBloom,
    receiptRoot: receiptTrie.root(),
    transactionRoot: transactionTrie.root(),
    blockNumber,
    blockHash,
    isPendingBlock,
  });
  store.push({
    collection: "headers",
    data: { header: ethHeader },
  });

  return store;
}
