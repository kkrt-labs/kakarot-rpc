import { RpcProvider } from "npm:starknet@5.24.3";
import { assertExists, assert, assertEquals, assertArrayIncludes } from "https://deno.land/std@0.213.0/assert/mod.ts";
import {
    BlockHeader,
    EventWithTransaction,
    TransactionWithReceipt,
    Event,
    LegacyTransaction,
} from "./deps.ts";
import transform from './main.ts';
import {
    TRANSACTION_EXECUTED,
} from "./constants.ts";
import { toTypedEthTx } from "./types/transaction.ts";

const provider = new RpcProvider({ nodeUrl: 'https://juno-kakarot-dev.karnot.xyz/' });
const targetCount = 100;
const transactions = await collectTransactions(targetCount);

async function fetchBlock(blockNumber: number) {
    const block = await provider.getBlock(blockNumber);
    return block;
}
async function collectTransactions(targetCount: number) {
    const transactionsList: any[] = [];
    const blocksList: any[] = [];
    let header: BlockHeader = {} as BlockHeader;
    let transactions: TransactionWithReceipt[] = [];
    let events: EventWithTransaction[] = [];

    let blockObj = await provider.getBlock(7500); // Should be "latest". with 7500 or lower works, with 8000 or higher it doesn't. (Invalid transaction: invalid RLP: total length is larger than the data)
    let blockNumber = blockObj.block_number;

    while (transactionsList.length < targetCount) {
        const block = await fetchBlock(blockNumber);
        console.log(`Block ${blockNumber} fetched, ${transactionsList.length} transactions collected.`)
        let getTransactionReceipts = await Promise.all(block.transactions.map(tx => provider.getTransactionReceipt(tx)));
        let getTransaction = await Promise.all(block.transactions.map(tx => provider.getTransaction(tx)));
        let transactionReceipts = getTransaction.map(tx => {
            let receipt = getTransactionReceipts.find(r => 'transaction_hash' in tx && r.transaction_hash === tx.transaction_hash);
            return { ...tx, ...receipt };
        });
        transactionsList.push(...transactionReceipts);
        blocksList.push(block);

        blockNumber -= 1;

        header = transformBlockHeader(block);
        const transformationResult = transformTransactionsAndEvents(transactionReceipts);
        transactions = transformationResult.transformedTransactions;
        events = transformationResult.eventsWithTransaction;

        if (blockNumber < 0) {
            throw new Error('Reached genesis block without collecting enough transactions.');
        }
    }

    return { header, events, transactions };
}

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

function transformTransactionsAndEvents(transactions: any[]): { transformedTransactions: TransactionWithReceipt[], eventsWithTransaction: EventWithTransaction[] } {
    const transformedTransactions: TransactionWithReceipt[] = [];
    const eventsWithTransaction: EventWithTransaction[] = [];

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
                calldata: tx.calldata,
            },
        };

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

        transformedTransactions.push({
            transaction,
            receipt,
        });

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

    return { transformedTransactions, eventsWithTransaction };
}

Deno.test("transform with real data", async () => {
    const result = await transform({ header: transactions.header, events: transactions.events, transactions: transactions.transactions });
    const collections = result.map(entry => entry.collection);
    const requiredCollections = ["transactions", "receipts", "logs", "headers"];
    assertExists(result);
    assert(result.length > 1);
    requiredCollections.forEach(collection => {
        assertArrayIncludes(collections, [collection], `${collection} is missing`);
    });
});

Deno.test("toTypedEthTx with real data", async () => {
    const ethTx = toTypedEthTx({ transaction: transactions.transactions[0].transaction }) as LegacyTransaction;
    assertExists(ethTx);
});
