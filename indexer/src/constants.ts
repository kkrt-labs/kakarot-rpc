import { padString } from "./utils/hex.ts";
import { hash } from "./deps.ts";

// Get Sink Type or returns "console" if the value is null or undefined
export const SINK_TYPE = (() => {
  const addr = Deno.env.get("SINK_TYPE") ?? "console";
  if (addr !== "console" && addr !== "mongo") {
    throw new Error("Invalid SINK_TYPE");
  }
  return addr;
})();

// Get the sink options from the sink type
export const SINK_OPTIONS = SINK_TYPE === "mongo"
  ? {
    connectionString: Deno.env.get("MONGO_CONNECTION_STRING") ??
      "mongodb://mongo:mongo@mongo:27017",
    database: Deno.env.get("MONGO_DATABASE_NAME") ?? "kakarot-test-db",
    collectionNames: ["headers", "transactions", "receipts", "logs"],
  }
  : {};

// Get the starting block or returns 0 if the value is null or undefined
export const STARTING_BLOCK = (() => {
  const startingBlock = Number(Deno.env.get("STARTING_BLOCK") ?? 0);
  return Number.isSafeInteger(startingBlock) && startingBlock >= 0
    ? startingBlock
    : (() => {
      throw new Error("Invalid STARTING_BLOCK");
    })();
})();

// Get stream URL or returns "http://localhost:7171" if the value is null or undefined
export const STREAM_URL = Deno.env.get("STREAM_URL") ?? "http://localhost:7171";

// Creates string that starts with "0x" and is padded to a total length of 64 chars
export const NULL_HASH = padString("0x", 32);

// Get the hash selector from the transaction executed
export const TRANSACTION_EXECUTED = hash.getSelectorFromName(
  "transaction_executed",
);

// Get the Kakarot Address 0x1
export const KAKAROT_ADDRESS: string = (() => {
  const kakarotAddress = Deno.env.get("KAKAROT_ADDRESS");
  if (!kakarotAddress) throw new Error("ENV: KAKAROT_ADDRESS is not set");
  return kakarotAddress;
})();
