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
  NULL_HASH,
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
import { toEthLog } from "./types/log.ts";
import { createTrieData } from "./types/tries.ts";
import { Collection, JsonRpcLog, StoreItem, TrieData } from "./types/types.ts";
// Starknet
import {
  BlockHeader,
  Config,
  EventWithTransaction,
  hexToBytes,
  JsonRpcTx,
  NetworkOptions,
  SinkOptions,
  TransactionWithReceipt,
} from "./deps.ts";
// Eth
import { Bloom, Trie } from "./deps.ts";
import {
  BlockInfo,
  ProcessedEvent,
  ProcessedTransaction,
} from "./types/interfaces.ts";

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
  const blockInfo = createBlockInfo(header);
  const store: Array<StoreItem> = [];
  const blockLogsBloom = new Bloom();

  const processedEvents = (events ?? [])
    // Can be false if the transaction is not related to a specific instance of the Kakarot contract.
    // This is typically the case if there are multiple Kakarot contracts on the same chain.
    // Skip if the transaction_executed event contains "eth validation failed".
    .filter(
      (event) =>
        isKakarotTransaction(event.transaction) &&
        !ethValidationFailed(event.event),
    )
    .map(processEvent(blockInfo))
    .filter((event): event is ProcessedEvent => event !== null);

  const { cumulativeGasUsed, cumulativeGasUsages } =
    accumulateGasAndUpdateStore(processedEvents, store, blockLogsBloom);

  const trieData: Array<TrieData> = processedEvents
    .map((event) =>
      createTrieData({
        transactionIndex: Number(event!.ethTx.transactionIndex),
        typedTransaction: event!.typedEthTx,
        receipt: event!.ethReceipt,
      }),
    )
    .filter((x) => x !== null);

  const { transactionTrie, receiptTrie } = await computeBlooms(trieData);

  // Sort the cumulative gas uses by descending transaction index.
  cumulativeGasUsages.reverse();

  const processedTransactions = processTransactions(
    transactions,
    blockInfo,
    cumulativeGasUsages,
  );
  updateStoreWithTransactions(store, processedTransactions);

  const ethHeader = await toEthHeader({
    header: header,
    gasUsed: cumulativeGasUsed,
    logsBloom: blockLogsBloom,
    receiptRoot: receiptTrie.root(),
    transactionRoot: transactionTrie.root(),
    ...blockInfo,
  });
  store.push({
    collection: Collection.Headers,
    data: { header: ethHeader },
  });

  return store;
}

function createBlockInfo(header: BlockHeader): BlockInfo {
  const blockNumber = padString(toHexString(header.blockNumber), 8);
  const blockHash = padString(header.blockHash, 32);
  const isPendingBlock = blockHash === NULL_HASH;
  return { blockNumber, blockHash, isPendingBlock };
}

function processEvent(blockInfo: BlockInfo) {
  return (event: EventWithTransaction): ProcessedEvent | null => {
    const typedEthTx = toTypedEthTx({ transaction: event.transaction });
    // Can be null if:
    // 1. The transaction is missing calldata.
    // 2. The transaction is a multi-call.
    // 3. The length of the signature array is different from 5.
    // 4. The chain id is not encoded in the v param of the signature for a
    //    Legacy transaction.
    // 5. The deserialization of the transaction fails.
    if (!typedEthTx) return null;

    const ethTx = typedTransactionToEthTx({
      typedTransaction: typedEthTx!,
      receipt: event.receipt,
      ...blockInfo,
    });
    // Can be null if:
    // The typed transaction is missing a signature param (v, r, s).
    if (!ethTx) return null;

    const ethLogs = event.receipt.events
      .map((e) => toEthLog({ transaction: ethTx, event: e, ...blockInfo }))
      // Can be null if:
      // 1. The event is part of the defined ignored events (see IGNORED_KEYS).
      // 2. The event has an invalid number of keys.
      .filter((e): e is JsonRpcLog => e !== null);

    const ethLogsIndexed = ethLogs.map((log, index) => ({
      ...log,
      logIndex: index.toString(),
    }));

    const ethReceipt = toEthReceipt({
      transaction: ethTx as JsonRpcTx,
      logs: ethLogsIndexed,
      event: event.event,
      cumulativeGasUsed: 0n, // This will be updated later
      ...blockInfo,
    });

    return { event, typedEthTx, ethTx, ethLogs: ethLogsIndexed, ethReceipt };
  };
}

function accumulateGasAndUpdateStore(
  processedEvents: ProcessedEvent[],
  store: Array<StoreItem>,
  blockLogsBloom: Bloom,
): { cumulativeGasUsed: bigint; cumulativeGasUsages: bigint[] } {
  let cumulativeGasUsed = 0n;
  const cumulativeGasUsages: bigint[] = [];

  processedEvents?.forEach((event, index) => {
    cumulativeGasUsed += BigInt(event.ethReceipt.gasUsed);
    cumulativeGasUsages[index] = cumulativeGasUsed;

    // Update the cumulative gas used in the receipt
    event.ethReceipt.cumulativeGasUsed = `0x${cumulativeGasUsed.toString(16)}`;

    store.push(
      ...[
        {
          collection: Collection.Transactions,
          data: { tx: event.ethTx },
        },
        {
          collection: Collection.Receipts,
          data: { receipt: event.ethReceipt },
        },
        ...event.ethLogs.map((log) => ({
          collection: Collection.Logs,
          data: { log },
        })),
      ],
    );
    updateBlockLogsBloom(blockLogsBloom, event);
  });

  return { cumulativeGasUsed, cumulativeGasUsages };
}

function updateBlockLogsBloom(blockLogsBloom: Bloom, event: ProcessedEvent) {
  const receiptLogsBloom = new Bloom(hexToBytes(event.ethReceipt.logsBloom));
  blockLogsBloom.or(receiptLogsBloom);
}

async function computeBlooms(
  trieData: Array<TrieData>,
): Promise<{ transactionTrie: Trie; receiptTrie: Trie }> {
  const transactionTrie = new Trie();
  const receiptTrie = new Trie();

  trieData.sort(
    (a, b) =>
      Number(a.encodedTransactionIndex) - Number(b.encodedTransactionIndex),
  );

  for (const {
    encodedTransactionIndex,
    encodedTransaction,
    encodedReceipt,
  } of trieData) {
    await transactionTrie.put(encodedTransactionIndex, encodedTransaction);
    await receiptTrie.put(encodedTransactionIndex, encodedReceipt);
  }

  return { transactionTrie, receiptTrie };
}

function processTransactions(
  transactions: TransactionWithReceipt[],
  blockInfo: BlockInfo,
  cumulativeGasUsages: bigint[],
): ProcessedTransaction[] {
  return (transactions ?? [])
    .filter(
      (tx) =>
        isRevertedWithOutOfResources(tx.receipt) &&
        isKakarotTransaction(tx.transaction),
    )
    .map((tx) => createProcessedTransaction(tx, blockInfo, cumulativeGasUsages))
    .filter((tx): tx is ProcessedTransaction => tx !== null);
}

function createProcessedTransaction(
  tx: TransactionWithReceipt,
  blockInfo: BlockInfo,
  cumulativeGasUsages: bigint[],
): ProcessedTransaction | null {
  if (!tx.transaction || !tx.receipt) return null;

  const ethTx = toEthTx({
    transaction: tx.transaction,
    receipt: tx.receipt,
    ...blockInfo,
  });
  if (!ethTx) return null;
  // Get the cumulative gas used for the reverted transaction.
  // Example:
  // const cumulativeGasUsages = [300n, undefined, undefined, 200n, undefined, 100n, undefined, undefined, 10n, undefined];
  // const ethTx = { transactionIndex: 5 };
  // const revertedTransactionCumulativeGasUsed = 100n;
  const revertedTransactionCumulativeGasUsed =
    cumulativeGasUsages.find(
      (gas, i) =>
        Number(ethTx.transactionIndex) >= cumulativeGasUsages.length - 1 - i &&
        gas,
    ) ?? 0n;

  const ethReceipt = toRevertedOutOfResourcesReceipt({
    transaction: ethTx as JsonRpcTx,
    cumulativeGasUsed: revertedTransactionCumulativeGasUsed,
    ...blockInfo,
  });

  return { ethTx, ethReceipt };
}

function updateStoreWithTransactions(
  store: Array<StoreItem>,
  processedTransactions: ProcessedTransaction[],
) {
  processedTransactions.forEach(({ ethTx, ethReceipt }) => {
    store.push({
      collection: Collection.Transactions,
      data: { tx: ethTx as JsonRpcTx },
    });
    store.push({
      collection: Collection.Receipts,
      data: { receipt: ethReceipt },
    });
  });
}
