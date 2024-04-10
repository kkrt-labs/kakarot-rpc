// Types
import { JsonRpcLog } from "./log.ts";
import { JsonRpcReceipt } from "./receipt.ts";

// Eth
import { JsonRpcTx } from "../deps.ts";
import { JsonRpcBlock } from "./header.ts";

type Collection =
  | "transactions"
  | "logs"
  | "receipts"
  | "headers";

export type StoreItem<C = Collection> = {
  collection: C;
  data: C extends "transactions" ? { tx: JsonRpcTx }
    : C extends "logs" ? { log: JsonRpcLog }
    : C extends "receipts" ? { receipt: JsonRpcReceipt }
    : { header: JsonRpcBlock };
};
