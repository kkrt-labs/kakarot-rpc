// Starknet
import { Transaction, Event } from "../deps.ts";

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
  const calldata = transaction.invokeV1?.calldata;
  if (calldata === undefined) {
    console.error("No calldata in transaction");
    console.error(JSON.stringify(transaction, null, 2));
    return false;
  }
  const to = calldata[1];
  if (to === undefined) {
    console.error("No `to` field in calldata of transaction");
    console.error(JSON.stringify(transaction, null, 2));
    return false;
  }
  // TODO(Greged93): replace this with a more robust check.
  // ⚠️ The existence of `to` field in invoke calldata in RPC is not enforced by protocol.
  // Forks or modifications of the kkrt-labs/kakarot-rpc codebase could break this check.
  if (BigInt(to) !== BigInt(KAKAROT_ADDRESS!)) {
    console.log("✅ Skipping transaction that is not related to Kakarot");
    return false;
  }
  return true;
};

export const ethValidationFailed = (event: Event) => {
  if (event.data.length === 0) {
    return false;
  }

  const response_len = parseInt(event.data[0], 16);

  if (response_len + 1 >= event.data.length) {
    console.error(
      `Invalid event data length. Got ${event.data.length}, expected < ${response_len + 1}`,
    );
    return false;
  }

  const success = parseInt(event.data[1 + response_len], 16);
  if (success == 1) {
    return false;
  }

  const response = event.data.slice(1, 1 + response_len);
  const msg = String.fromCharCode(...response.map((x) => parseInt(x, 16)));

  return msg.includes("eth validation failed");
};
