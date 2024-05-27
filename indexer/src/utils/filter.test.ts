import {
  assert,
  assertFalse,
} from "https://deno.land/std@0.213.0/assert/mod.ts";
import { ethValidationFailed } from "./filter.ts";
import { Event } from "../deps.ts";

const event = (data: `0x${string}`[]) => {
  return {
    index: 1,
    fromAddress: "0x1",
    keys: ["0x1", "0x2", "0x3"],
    data,
  } as Event;
};

Deno.test("ethValidationFailed: failed transaction", () => {
  const x = event([
    "0x1e",
    "0x4b",
    "0x61",
    "0x6b",
    "0x61",
    "0x72",
    "0x6f",
    "0x74",
    "0x3a",
    "0x20",
    "0x65",
    "0x74",
    "0x68",
    "0x20",
    "0x76",
    "0x61",
    "0x6c",
    "0x69",
    "0x64",
    "0x61",
    "0x74",
    "0x69",
    "0x6f",
    "0x6e",
    "0x20",
    "0x66",
    "0x61",
    "0x69",
    "0x6c",
    "0x65",
    "0x64",
    "0x0",
    "0x0",
  ]);
  const failed = ethValidationFailed(x);
  assert(failed);
});

Deno.test("ethValidationFailed: empty data", () => {
  const x = event([]);
  const failed = ethValidationFailed(x);
  assertFalse(failed);
});

Deno.test("ethValidationFailed: success true", () => {
  const x = event(["0x1", "0x1", "0x1", "0x1"]);
  const failed = ethValidationFailed(x);
  assertFalse(failed);
});

Deno.test("ethValidationFailed: incorrect data length", () => {
  const x = event(["0x10"]);
  const failed = ethValidationFailed(x);
  assertFalse(failed);
});
