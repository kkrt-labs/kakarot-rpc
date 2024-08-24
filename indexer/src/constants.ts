import { padString } from "./utils/hex.ts";
import { hash } from "./deps.ts";

const getEnvVariable = <T>(key: string, defaultValue: T, validator?: (value: T) => boolean): T => {
  const value = Deno.env.get(key) as T ?? defaultValue;
  if (validator && !validator(value)) {
    throw new Error(`Invalid ${key}`);
  }
  return value;
};

export const SINK_TYPE = getEnvVariable("SINK_TYPE", "console", (value) => ["console", "mongo"].includes(value as string));

export const SINK_OPTIONS = SINK_TYPE === "mongo" ? {
  connectionString: getEnvVariable("MONGO_CONNECTION_STRING", "mongodb://mongo:mongo@mongo:27017"),
  database: getEnvVariable("MONGO_DATABASE_NAME", "kakarot-test-db"),
  collectionNames: ["headers", "transactions", "receipts", "logs"],
} : {};

export const STARTING_BLOCK = getEnvVariable("STARTING_BLOCK", 0, (value) => 
  Number.isSafeInteger(Number(value)) && Number(value) >= 0
);

export const AUTH_TOKEN = getEnvVariable("APIBARA_AUTH_TOKEN", "");
export const STREAM_URL = getEnvVariable("STREAM_URL", "http://localhost:7171");
export const NULL_BLOCK_HASH = padString("0x", 32);
export const TRANSACTION_EXECUTED = hash.getSelectorFromName("transaction_executed");
export const KAKAROT_ADDRESS = getEnvVariable("KAKAROT_ADDRESS", "", (value) => !!value);
