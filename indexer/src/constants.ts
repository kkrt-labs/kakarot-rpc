import { padString } from "./utils/hex.ts";
import { hash } from "./deps.ts";

// Get Sink Type or returns "console" if the value is null or undefined
export const SINK_TYPE = (() => {
    const addr = Deno.env.get("SINK_TYPE") ?? "console";
    if (addr !== "console" && addr !== "mongo") throw new Error("Invalid SINK_TYPE");
    return addr;
})();

// Get authentication token from Apibara or returns an empty string if the value is null or undefined
export const AUTH_TOKEN = Deno.env.get("APIBARA_AUTH_TOKEN") ?? "";

// Get stream URL or returns "http://localhost:7171" if the value is null or undefined
export const STREAM_URL = Deno.env.get("STREAM_URL") ?? "http://localhost:7171";

// Get the starting block or returns 0 if the value is null or undefined
export const STARTING_BLOCK = (() => {
    const addr = Number(Deno.env.get("STARTING_BLOCK")) ?? 0;
    if (!Number.isSafeInteger(addr) || addr < 0) throw new Error("Invalid STARTING_BLOCK");
    return addr;
})();

// Creates string that starts with "0x" and is padded to a total lenght of 64 chars
export const NULL_BLOCK_HASH = padString("0x", 32);

// Get the hash selector from the transaction executed
export const TRANSACTION_EXECUTED = hash.getSelectorFromName("transaction_executed");

// Get the Kakarot Address 0x1
export const KAKAROT_ADDRESS: string = (() => {
    const addr = Deno.env.get("KAKAROT_ADDRESS");
    if (!addr) throw new Error("ENV: KAKAROT_ADDRESS is not set");
    return addr;
})();

// Get the URL of the Starknet Network
export const RPC_URL = (() => {
    const addr = Deno.env.get("STARKNET_NETWORK");
    if (!addr) throw new Error("ENV: STARKNET_NETWORK is not set");
    return addr;
})();

// A default block gas limit in case the call to get_block_gas_limit fails.
export const DEFAULT_BLOCK_GAS_LIMIT = (() => {
    const addr = Deno.env.get("DEFAULT_BLOCK_GAS_LIMIT");
    if (!addr) throw new Error("ENV: DEFAULT_BLOCK_GAS_LIMIT is not set");
    return addr;
})();
