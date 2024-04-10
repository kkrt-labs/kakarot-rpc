import { assertEquals } from "https://deno.land/std@0.213.0/assert/mod.ts";
import { padBigint, padBytes, padString, toHexString } from "./hex.ts";

Deno.test("toHexString #1", () => {
  const x = "1234";
  const expected = "0x4d2";
  const paddedX = toHexString(x);
  assertEquals(paddedX, expected);
});

Deno.test("toHexString #2", () => {
  const x = undefined;
  const expected = "0x";
  const paddedX = toHexString(x);
  assertEquals(paddedX, expected);
});

Deno.test("padString #1", () => {
  const x = "0x010203";
  const expected = "0x0000000000010203";
  const paddedX = padString(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padString #2", () => {
  const x = undefined;
  const expected = "0x0000000000000000";
  const paddedX = padString(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint #1", () => {
  const x = BigInt("0x010203");
  const expected = "0x0000000000010203";
  const paddedX = padBigint(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint #2", () => {
  const x = undefined;
  const expected = "0x0000000000000000";
  const paddedX = padBigint(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBytes #1", () => {
  const x = new Uint8Array([1, 2, 3]);
  const expected = "0x0000000000010203";
  const paddedX = padBytes(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBytes #2", () => {
  const x = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9]);
  const expected = "0x010203040506070809";
  const paddedX = padBytes(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBytes #3", () => {
  const x = undefined;
  const expected = "0x0000000000000000";
  const paddedX = padBytes(x, 8);
  assertEquals(paddedX, expected);
});
