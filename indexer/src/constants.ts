import { padString } from "./utils/hex.ts";
import { hash } from "./deps.ts";

// Helper function to get environment variables with validation
const getEnvVariable = <T>(
  key: string,
  defaultValue: T,
  validator?: (value: T) => boolean,
): T => {
  const value = Deno.env.get(key) as T ?? defaultValue;
  if (validator && !validator(value)) {
    throw new Error(`Invalid ${key}`);
  }
  return value;
};

// Define the sink type (console or mongo) from environment variable
export const SINK_TYPE = getEnvVariable(
  "SINK_TYPE",
  "console",
  (value) => ["console", "mongo"].includes(value as string),
);

// Set sink options based on the sink type
export const SINK_OPTIONS = SINK_TYPE === "mongo"
  ? {
    connectionString: getEnvVariable(
      "MONGO_CONNECTION_STRING",
      "mongodb://mongo:mongo@mongo:27017",
    ),
    database: getEnvVariable("MONGO_DATABASE_NAME", "kakarot-test-db"),
    collectionNames: ["headers", "transactions", "receipts", "logs"],
  }
  : {};

// Get the starting block or returns 0 if the value is null or undefined
export const STARTING_BLOCK = getEnvVariable(
  "STARTING_BLOCK",
  0,
  (value) => Number.isSafeInteger(Number(value)) && Number(value) >= 0,
);

// Get authentication token from Apibara or returns an empty string if the value is null or undefined
export const AUTH_TOKEN = getEnvVariable("APIBARA_AUTH_TOKEN", "");

// Get stream URL or returns "http://localhost:7171" if the value is null or undefined
export const STREAM_URL = getEnvVariable("STREAM_URL", "http://localhost:7171");

// Creates string that starts with "0x" and is padded to a total length of 64 chars
export const NULL_BLOCK_HASH = padString("0x", 32);

// Get the selector for "transaction_executed" event
export const TRANSACTION_EXECUTED = hash.getSelectorFromName(
  "transaction_executed",
);

// Get the Kakarot address from environment variable (0x1)
export const KAKAROT_ADDRESS = getEnvVariable(
  "KAKAROT_ADDRESS",
  "",
  (value) => !!value,
);
