import { RpcProvider } from "npm:starknet@5.24.3";
import {
  BlockHeader,
  Event,
  EventWithTransaction,
  PrefixedHexString,
  Transaction,
  TransactionWithReceipt,
} from "../indexer/src/deps.ts";
import { TRANSACTION_EXECUTED } from "../indexer/src/constants.ts";
import {
  EXPECTED_TRANSFORM_DATA_FILE,
  TRANSACTIONS_DATA_FILE,
} from "../indexer/src/testConstants.ts";
import { padString } from "../indexer/src/utils/hex.ts";
import transform from "../indexer/src/main.ts";
import { toTypedEthTx } from "../indexer/src/types/transaction.ts";

// Initialize the provider with the specified node URL
const provider = new RpcProvider({
  nodeUrl: "https://juno-kakarot-dev.karnot.xyz/",
});

// Function to fetch a block by its number
async function fetchBlock(blockNumber: number) {
  const block = await provider.getBlock(blockNumber);
  return block;
}

// Function to collect transactions until a target count is reached
async function collectTransactions(targetCount: number) {
  const transactionsList: any[] = [];
  const eventsList: any[] = [];
  const headersList: any[] = [];
  const expectedTransform: any[] = [];
  let header: BlockHeader = {} as BlockHeader;
  let transactions: TransactionWithReceipt[] = [];
  let events: EventWithTransaction[] = [];
  let expectedToTypedEthTxTransactions: Transaction[] = [];

  // Fetch the latest block to start the process
  let blockObj = await provider.getBlock("latest");
  let blockNumber = blockObj.block_number;

  // Loop to collect transactions until the target count is met
  while (transactionsList.length < targetCount) {
    const block = await fetchBlock(blockNumber);
    console.log(
      `Block ${blockNumber} fetched, ${transactionsList.length} transactions collected.`,
    );
    let getTransactionReceipts = await Promise.all(
      block.transactions.map((tx) => provider.getTransactionReceipt(tx)),
    );
    let getTransaction = await Promise.all(
      block.transactions.map((tx) => provider.getTransaction(tx)),
    );

    // Combine transaction data with receipts
    let transactionReceipts = getTransaction.map((tx) => {
      let receipt = getTransactionReceipts.find(
        (r) =>
          "transaction_hash" in tx &&
          r.transaction_hash === tx.transaction_hash,
      );
      return { ...tx, ...receipt };
    });

    blockNumber -= 1;

    // Transform block header and transactions
    header = transformBlockHeader(block);
    const {
      transformedTransactions,
      eventsWithTransaction,
      toTypedEthTxTransaction,
    } = transformTransactionsAndEvents(transactionReceipts);
    transactions = transformedTransactions;
    events = eventsWithTransaction;

    // Transform the data for storage or further processing
    const transforResult = await transform({ header, events, transactions });

    // Add the collected data to the respective lists
    transactionsList.push(transactions);
    headersList.push(header);
    eventsList.push(events);
    expectedTransform.push(transforResult);
    expectedToTypedEthTxTransactions.push(toTypedEthTxTransaction);

    // Check if the genesis block is reached
    if (blockNumber < 0) {
      throw new Error(
        "Reached genesis block without collecting enough transactions.",
      );
    }
  }

  return {
    headersList,
    eventsList,
    transactionsList,
    expectedTransform,
    expectedToTypedEthTxTransactions,
  };
}

// Function to transform block header data into the desired format
function transformBlockHeader(block: any): BlockHeader {
  return {
    blockNumber: block.block_number,
    blockHash: block.block_hash,
    parentBlockHash: block.parent_hash,
    newRoot: block.new_root,
    timestamp: block.timestamp,
    sequencerAddress: block.sequencer_address ? block.sequencer_address : null,
  };
}

// Function to transform transactions and events
function transformTransactionsAndEvents(transactions: any[]): {
  transformedTransactions: TransactionWithReceipt[];
  eventsWithTransaction: EventWithTransaction[];
  toTypedEthTxTransaction: Transaction[];
} {
  const transformedTransactions: TransactionWithReceipt[] = [];
  const eventsWithTransaction: EventWithTransaction[] = [];
  const toTypedEthTxTransaction: Transaction[] = [];

  // Iterate through each transaction to transform it and extract events
  transactions.forEach((tx: any, txIndex: number) => {
    const transaction = {
      meta: {
        hash: tx.transaction_hash,
        maxFee: tx.max_fee,
        signature: tx.signature,
        nonce: tx.nonce,
        version: tx.version,
      },
      invokeV1: {
        senderAddress: tx.sender_address,
        calldata: tx.calldata.map((x: PrefixedHexString) => padString(x, 32)),
      },
    };

    // Convert to typed Ethereum transaction
    const typedEthTx = toTypedEthTx({ transaction });

    const receipt = {
      executionStatus: tx.execution_status,
      transactionHash: tx.transaction_hash,
      transactionIndex: txIndex.toString(),
      actualFee: tx.actual_fee,
      contractAddress: tx.contractAddress,
      l2ToL1Messages: tx.messages_sent,
      events: tx.events.map((evt: any, evtIndex: number) => ({
        fromAddress: evt.from_address,
        keys: evt.keys,
        data: evt.data,
        index: evtIndex,
      })),
    };

    // Add the transformed transaction and receipt to the list
    transformedTransactions.push({
      transaction,
      receipt,
    });

    toTypedEthTxTransaction.push(typedEthTx);

    // Extract events related to the transaction
    tx.events.forEach((ev: any, eventIndex: number) => {
      if (ev.keys[0] === TRANSACTION_EXECUTED) {
        const event: Event = {
          fromAddress: ev.from_address,
          keys: ev.keys,
          data: ev.data,
          index: eventIndex,
        };

        eventsWithTransaction.push({ transaction, receipt, event });
      }
    });
  });

  return {
    transformedTransactions,
    eventsWithTransaction,
    toTypedEthTxTransaction,
  };
}

async function main() {
  try {
    const targetCount = 100;
    const {
      headersList,
      eventsList,
      transactionsList,
      expectedTransform,
      expectedToTypedEthTxTransactions,
    } = await collectTransactions(targetCount);

    // Save the collected transactions to a file
    await Deno.writeTextFile(
      TRANSACTIONS_DATA_FILE,
      JSON.stringify({ headersList, eventsList, transactionsList }, null, 2),
    );
    console.log("Transactions saved to transactions.json");

    // Save the expected transformed data to a file
    await Deno.writeTextFile(
      EXPECTED_TRANSFORM_DATA_FILE,
      JSON.stringify(
        { expectedTransform, expectedToTypedEthTxTransactions },
        null,
        2,
      ),
    );
    console.log("Expected data saved to expectedTransformData.json");
  } catch (error) {
    console.error("Error collecting transactions:", error);
  }
}

await main();
