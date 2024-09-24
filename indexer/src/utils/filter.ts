// Starknet
import { Event, Transaction, TransactionReceipt } from "../deps.ts";

// Constants
import { KAKAROT_ADDRESS } from "../constants.ts";

/**
 * Determines if a given transaction is related to Kakarot.
 *
 * This function checks the calldata of the transaction to see if the
 * `to` field matches the KAKAROT_ADDRESS. The calldata structure is
 * expected to follow a specific format:
 * - callLen <- calldata[0]
 * - to <- calldata[1]
 * - selector <- calldata[2]
 * - calldataLen <- calldata[3]
 * - OutsideExecution <- calldata[4..=7] skip
 * - callArrayLen <- calldata[8]
 * - to <- calldata[9]
 *
 * @param {Transaction} transaction - The transaction to check.
 * @returns {boolean} - Returns true if the transaction is related to Kakarot, otherwise false.
 */
export function isKakarotTransaction(transaction: Transaction): boolean {
  return (
    BigInt(transaction.invokeV1?.calldata?.[9] ?? 0) === BigInt(KAKAROT_ADDRESS)
  );
}

/**
 * Validates if an Ethereum validation has failed based on the event data.
 *
 * @param {Event} event - The event containing data to validate.
 * @returns {boolean} - Returns true if the validation failed, otherwise false.
 */
export function ethValidationFailed(event: Event): boolean {
  // We only need the data array to validate the event
  const { data } = event;

  // If data array is empty, return false (no validation failure)
  if (!data.length) return false;

  // Parse the first element of data as the response length in hexadecimal
  const responseLen = parseInt(data[0] ?? "", 16);

  // Check if the data length is valid (greater than response length + 1)
  const isValidLength = data.length > responseLen + 1;

  // If data length is invalid, log an error and return false
  if (!isValidLength) {
    console.error(
      `Invalid event data length. Got ${data.length}, expected > ${
        responseLen + 1
      }`,
    );
    return false;
  }

  // Parse the element at position (response length + 1) as success flag in hexadecimal
  const success = parseInt(data[responseLen + 1] ?? "", 16);

  // If success flag is set (1), return false (no validation failure)
  if (success == 1) return false;

  // Extract the response data slice, convert it to a string, and check if it includes "eth validation failed"
  return String.fromCharCode(
    ...data.slice(1, 1 + responseLen).map((x) => parseInt(x, 16)),
  ).includes("eth validation failed");
}

/**
 * Checks if a transaction receipt indicates that it was reverted due to running out of resources.
 *
 * @param {TransactionReceipt} receipt - The transaction receipt to check.
 * @returns {boolean} - Returns true if the transaction was reverted due to out of resources, otherwise false.
 */
export function isRevertedWithOutOfResources(
  receipt: TransactionReceipt,
): boolean {
  return (
    receipt.executionStatus.includes("REVERTED") &&
    (receipt.revertReason ?? "").includes("RunResources has no remaining steps")
  );
}
