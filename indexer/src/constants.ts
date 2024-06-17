import { padString } from "./utils/hex.ts";
import { hash } from "./deps.ts";

export const NULL_BLOCK_HASH = padString("0x", 32);
export const TRANSACTION_EXECUTED = hash.getSelectorFromName("transaction_executed");
export const KAKAROT_ADDRESS = Deno.env.get("KAKAROT_ADDRESS");
export const AUTH_TOKEN = Deno.env.get("APIBARA_AUTH_TOKEN") ?? "";
export const STREAM_URL = Deno.env.get("STREAM_URL") ?? "http://localhost:7171";
export const STARTING_BLOCK = Number(Deno.env.get("STARTING_BLOCK")) ?? 0;
export const SINK_TYPE = Deno.env.get("SINK_TYPE") ?? "console";
export const DEFAULT_BLOCK_GAS_LIMIT = Deno.env.get("DEFAULT_BLOCK_GAS_LIMIT");
export const RPC_URL = Deno.env.get("STARKNET_NETWORK");
