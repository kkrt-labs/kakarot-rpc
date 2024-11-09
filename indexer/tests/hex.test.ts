import { assertEquals } from "https://deno.land/std@0.213.0/assert/mod.ts";
import {
  padBigint,
  padBytes,
  padString,
  toHexString,
} from "../src/utils/hex.ts";

Deno.test("toHexString converts string to hex format", () => {
  const x = "1234";
  const expected = "0x4d2";
  const paddedX = toHexString(x);
  assertEquals(paddedX, expected);
});

Deno.test("toHexString handles undefined input", () => {
  const x = undefined;
  const expected = "0x";
  const paddedX = toHexString(x);
  assertEquals(paddedX, expected);
});

Deno.test("padString pads hex string to specified length", () => {
  const x = "0x010203";
  const expected = "0x0000000000010203";
  const paddedX = padString(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test(
  "padString does not pad when hex string length is equal to specified length",
  () => {
    const x = "0x010203";
    const expected = "0x010203";
    const paddedX = padString(x, 3);
    assertEquals(paddedX, expected);
  },
);

Deno.test(
  "padString handles hex string with length equal to specified length",
  () => {
    const x = "0x010203";
    const expected = "0x010203";
    const paddedX = padString(x, 2);
    assertEquals(paddedX, expected);
  },
);

Deno.test("padString pads hex string to specified length", () => {
  const x = "0x0000000000010203";
  const expected = "0x0000000000010203";
  const paddedX = padString(x, 4);
  assertEquals(paddedX, expected);
});

Deno.test("padString handles undefined input", () => {
  const x = undefined;
  const expected = "0x0000000000000000";
  const paddedX = padString(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint pads bigint to specified length", () => {
  const x = BigInt("0x010203");
  const expected = "0x0000000000010203";
  const paddedX = padBigint(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint handles undefined input", () => {
  const x = undefined;
  const expected = "0x0000000000000000";
  const paddedX = padBigint(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint with zero bigint to specified length", () => {
  const x = BigInt("0x0");
  const expected = "0x0000000000000000";
  const paddedX = padBigint(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint does not pad if hex length >= specified length", () => {
  const x = BigInt("0x010203040506070809");
  const expected = "0x10203040506070809";
  const paddedX = padBigint(x, 4);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint handles smaller bigint and shorter padding length", () => {
  const x = BigInt("0x000000000000009");
  const expected = "0x00000009";
  const paddedX = padBigint(x, 4);
  assertEquals(paddedX, expected);
});

Deno.test("padBigint does not pad when length is equal", () => {
  const x = BigInt("0x0000000000000009");
  const expected = "0x0000000000000009";
  const paddedX = padBigint(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test("padBytes pads Uint8Array to specified length", () => {
  const x = new Uint8Array([1, 2, 3]);
  const expected = "0x0000000000010203";
  const paddedX = padBytes(x, 8);
  assertEquals(paddedX, expected);
});

Deno.test(
  "padBytes pads Uint8Array with larger data to specified length",
  () => {
    const x = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9]);
    const expected = "0x010203040506070809";
    const paddedX = padBytes(x, 8);
    assertEquals(paddedX, expected);
  },
);

Deno.test(
  "padBytes pads Uint8Array with larger data to specified length (and zeros",
  () => {
    const x = new Uint8Array([0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    const expected = "0x0000010203040506070809";
    const paddedX = padBytes(x, 8);
    assertEquals(paddedX, expected);
  },
);

Deno.test("padBytes pads Uint8Array to exact specified length", () => {
  const x = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9]);
  const expected = "0x010203040506070809";
  const paddedX = padBytes(x, 9);
  assertEquals(paddedX, expected);
});

Deno.test("padBytes pads Uint8Array empty to specified length", () => {
  const x = new Uint8Array([]);
  const expected = "0x000000000000000000";
  const paddedX = padBytes(x, 9);
  assertEquals(paddedX, expected);
});

Deno.test("padBytes handles undefined input", () => {
  const x = undefined;
  const expected = "0x0000000000000000";
  const paddedX = padBytes(x, 8);
  assertEquals(paddedX, expected);
});
