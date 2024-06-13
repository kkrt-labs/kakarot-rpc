// Starknet
import { Event, Transaction, TransactionReceipt } from "../deps.ts";

const KAKAROT_ADDRESS = Deno.env.get("KAKAROT_ADDRESS");
if (KAKAROT_ADDRESS === undefined) {
  throw new Error("ENV: KAKAROT_ADDRESS is not set");
}

export const isKakarotTransaction = (transaction: Transaction) => {
  // Filter out transactions that are not related to Kakarot.
  // callArrayLen <- calldata[0]
  // to <- calldata[1]
  // selector <- calldata[2];
  // dataOffset <- calldata[3]
  // dataLength <- calldata[4]
  // calldataLen <- calldata[5]
  // signedDataLen <- calldata[6]
  const calldata = transaction.invokeV1?.calldata;
  if (!calldata) {
    console.error("No calldata in transaction");
    console.error(JSON.stringify(transaction, null, 2));
    return false;
  }
  const to = calldata[1];
  if (!to) {
    console.error("No `to` field in calldata of transaction");
    console.error(JSON.stringify(transaction, null, 2));
    return false;
  }

  if (BigInt(to) !== BigInt(KAKAROT_ADDRESS)) {
    console.log("✅ Skipping transaction that is not related to Kakarot");
    return false;
  }
  return true;
};

export const ethValidationFailed = (event: Event) => {
  const { data } = event;
  if (data.length === 0) {
    return false;
  }

  const response_len = parseInt(data[0], 16);

  if (response_len + 1 >= data.length) {
    console.error(
      `Invalid event data length. Got ${data.length}, expected < ${
        response_len + 1
      }`,
    );
    return false;
  }

  const success = parseInt(data[1 + response_len], 16);
  if (success == 1) {
    return false;
  }

  const response = data.slice(1, 1 + response_len);
  const msg = String.fromCharCode(...response.map((x) => parseInt(x, 16)));

  return msg.includes("eth validation failed");
};

export const isRevertedWithOutOfResources = (receipt: TransactionReceipt) => {
  return (
    receipt.executionStatus.includes("REVERTED") &&
    receipt.revertReason?.includes("RunResources has no remaining steps")
  );
};
