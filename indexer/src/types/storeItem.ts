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
  | "headers"
  | "transactions_failure";

export type StoreItem<C = Collection> = {
  collection: C;
  data: C extends "transactions"
    ? { tx: JsonRpcTx }
    : C extends "logs"
      ? { log: JsonRpcLog }
      : C extends "receipts"
        ? { receipt: JsonRpcReceipt }
        : C extends "transactions_failure"
          ? { tx: JsonRpcTx }
          : { header: JsonRpcBlock };
};
