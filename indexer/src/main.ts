// Utils
import { padString, toHexString } from "./utils/hex.ts";
import {
  ethValidationFailed,
  isKakarotTransaction,
  isRevertedWithOutOfResources,
} from "./utils/filter.ts";

// Constants
import {
  AUTH_TOKEN,
  NULL_BLOCK_HASH,
  SINK_OPTIONS,
  SINK_TYPE,
  STARTING_BLOCK,
  STREAM_URL,
  TRANSACTION_EXECUTED,
} from "./constants.ts";

// Types
import {
  toEthTx,
  toTypedEthTx,
  typedTransactionToEthTx,
} from "./types/transaction.ts";
import { toEthHeader } from "./types/header.ts";
import {
  toEthReceipt,
  toRevertedOutOfResourcesReceipt,
} from "./types/receipt.ts";
import { JsonRpcLog, toEthLog } from "./types/log.ts";
import { createTrieData, TrieData } from "./types/tries.ts";
import { StoreItem } from "./types/storeItem.ts";
// Starknet
import {
  BlockHeader,
  Config,
  EventWithTransaction,
  hexToBytes,
  NetworkOptions,
  SinkOptions,
  TransactionWithReceipt,
} from "./deps.ts";
// Eth
import { Bloom, Trie } from "./deps.ts";

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
    transactions: [{ includeReverted: true }],
  },
  sinkType: SINK_TYPE,
  sinkOptions: SINK_OPTIONS,
};

export default async function transform({
  header,
  events,
  transactions,
}: {
  header: BlockHeader;
  events: EventWithTransaction[];
  transactions: TransactionWithReceipt[];
}) {
  // Accumulate the gas used in the block in order to calculate the cumulative gas used.
  // We increment it by the gas used in each transaction in the flatMap iteration.
  let cumulativeGasUsed = 0n;
  // An array containing the cumulative gas used up to that transaction, indexed by
  // transaction index. This is used to later get the cumulative gas used for an out of
  // resources transaction.
  const cumulativeGasUsages: Array<bigint> = [];

  const blockNumber = padString(toHexString(header.blockNumber), 8);
  const isPendingBlock = padString(header.blockHash, 32) === NULL_BLOCK_HASH;
  const blockHash = padString(header.blockHash, 32);
  const blockLogsBloom = new Bloom();
  const transactionTrie = new Trie();
  const receiptTrie = new Trie();

  const store: Array<StoreItem> = [];

  const maybeTrieData: Array<TrieData | null> = (events ?? []).map(
    ({ transaction, receipt, event }) => {
      console.log(
        "🔍 Processing transaction with Starknet hash: ",
        transaction.meta.hash,
      );
      // Can be false if the transaction is not related to a specific instance of the Kakarot contract.
      // This is typically the case if there are multiple Kakarot contracts on the same chain.
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
      if (!typedEthTx) {
        return null;
      }
      const ethTx = typedTransactionToEthTx({
        typedTransaction: typedEthTx,
        receipt,
        blockNumber,
        blockHash,
        isPendingBlock,
      });
      // Can be null if:
      // 1. The typed transaction if missing a signature param (v, r, s).
      if (!ethTx) {
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

      cumulativeGasUsed += BigInt(ethReceipt.gasUsed);
      // ethTx.transactionIndex can be null (if the block is pending) but
      // Number(null) is 0 so this won't panic.
      cumulativeGasUsages[Number(ethTx.transactionIndex)] = cumulativeGasUsed;

      // Add all the eth data to the store.
      store.push({ collection: "transactions", data: { tx: ethTx } });
      store.push({ collection: "receipts", data: { receipt: ethReceipt } });
      ethLogs.forEach((ethLog) => {
        store.push({ collection: "logs", data: { log: ethLog } });
      });

      // Add the logs bloom of the receipt to the block logs bloom.
      const receiptLogsBloom = new Bloom(hexToBytes(ethReceipt.logsBloom));
      blockLogsBloom.or(receiptLogsBloom);

      /// Return the trie data.
      return createTrieData({
        transactionIndex: Number(ethTx.transactionIndex),
        typedTransaction: typedEthTx,
        receipt: ethReceipt,
      });
    },
  );

  // Filter out the null values for the trie data.
  const trieData = maybeTrieData.filter((x) => x !== null) as Array<TrieData>;

  // Compute the blooms in an async manner.
  await Promise.all(
    trieData.map(
      async ({
        encodedTransactionIndex,
        encodedTransaction,
        encodedReceipt,
      }) => {
        // Add the transaction to the transaction trie.
        await transactionTrie.put(encodedTransactionIndex, encodedTransaction);
        // Add the receipt to the receipt trie.
        await receiptTrie.put(encodedTransactionIndex, encodedReceipt);
      },
    ),
  );

  // Sort the cumulative gas uses by descending transaction index.
  cumulativeGasUsages.reverse();

  (transactions ?? []).forEach(({ transaction, receipt }) => {
    if (isRevertedWithOutOfResources(receipt)) {
      // Can be false if the transaction is not related to a specific instance of the Kakarot contract.
      // This is typically the case if there are multiple Kakarot contracts on the same chain.
      const isKakarotTx = isKakarotTransaction(transaction);
      if (!isKakarotTx) {
        return;
      }

      const ethTx = toEthTx({
        transaction,
        receipt,
        blockNumber,
        blockHash,
        isPendingBlock,
      });
      if (!ethTx) {
        return;
      }

      // Get the cumulative gas used for the reverted transaction.
      // Example:
      // const cumulativeGasUsages = [300n, undefined, undefined, 200n, undefined, 100n, undefined, undefined, 10n, undefined];
      // const ethTx = { transactionIndex: 5 };
      // const revertedTransactionCumulativeGasUsed = 100n;
      const len = cumulativeGasUsages.length;
      const revertedTransactionCumulativeGasUsed =
        cumulativeGasUsages.find((gas, i) => {
          return (
            Number(ethTx.transactionIndex) >= len - 1 - i && gas !== undefined
          );
        }) ?? 0n;

      const ethReceipt = toRevertedOutOfResourcesReceipt({
        transaction: ethTx,
        blockNumber,
        blockHash,
        cumulativeGasUsed: revertedTransactionCumulativeGasUsed,
        isPendingBlock,
      });

      store.push({ collection: "transactions", data: { tx: ethTx } });
      store.push({
        collection: "receipts",
        data: {
          receipt: ethReceipt,
        },
      });
    }
  });

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
