// Eth
import { bigIntToHex, bytesToHex, PrefixedHexString, stripHexPrefix } from "../deps.ts";

export const NULL_BLOCK_HASH = padString("0x", 32);

/**
 * @param hex - A decimal string.
 */
export function toHexString(decimal: string | undefined): PrefixedHexString {
  return decimal ? bigIntToHex(BigInt(decimal)) : "0x";
}

/**
 * @param hex - A hex string.
 * @param length - The final length in bytes of the hex string.
 */
export function padString(
  hex: PrefixedHexString | undefined,
  length: number,
): PrefixedHexString {
  return "0x" + stripHexPrefix(hex ?? "0x").padStart(2 * length, "0");
}

/**
 * @param b - A bigint.
 * @param length - The final length in bytes of the hex string.
 */
export function padBigint(
  b: bigint | undefined,
  length: number,
): PrefixedHexString {
  return "0x" + stripHexPrefix(bigIntToHex(b ?? 0n)).padStart(2 * length, "0");
}

/**
 * @param bytes - A Uint8Array.
 * @param length - The final length in bytes of the array. If
 * the array is longer than the length, it is returned as is.
 */
export function padBytes(
  maybeBytes: Uint8Array | undefined,
  length: number,
): PrefixedHexString {
  const bytes = maybeBytes ?? new Uint8Array();
  if (bytes.length > length) {
    return bytesToHex(bytes);
  }
  const result = new Uint8Array(length);
  result.set(bytes, length - bytes.length);
  return bytesToHex(result);
}
