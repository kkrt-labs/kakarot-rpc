// Types
import { JsonRpcLog } from "./log.ts";
import { JsonRpcReceipt } from "./receipt.ts";

// Eth
import { JsonRpcTx } from "../deps.ts";
import { JsonRpcBlock } from "./header.ts";

export enum Collection {
  Transactions = "transactions",
  Logs = "logs",
  Receipts = "receipts",
  Headers = "headers",
}

export type StoreItem<C = Collection> = {
  collection: C;
  data: C extends Collection.Transactions ? { tx: JsonRpcTx }
    : C extends Collection.Logs ? { log: JsonRpcLog }
    : C extends Collection.Receipts ? { receipt: JsonRpcReceipt }
    : { header: JsonRpcBlock };
};
