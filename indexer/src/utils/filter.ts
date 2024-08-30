// Starknet
import { Event, Transaction, TransactionReceipt } from "../deps.ts";

// Constants
import { KAKAROT_ADDRESS } from "../constants.ts";

/**
 * Checks if a given transaction is related to Kakarot.
 *
 * This function inspects the calldata of the transaction to verify if
 * the 'to' field matches the predefined KAKAROT_ADDRESS. The calldata
 * structure is expected to follow this format:
 * - callArrayLen <- calldata[0]
 * - to <- calldata[1]
 * - selector <- calldata[2]
 * - dataOffset <- calldata[3]
 * - dataLength <- calldata[4]
 * - calldataLen <- calldata[5]
 * - signedDataLen <- calldata[6]
 *
 * @param {Transaction} transaction - The transaction to check.
 * @returns {boolean} - Returns true if the transaction is related to Kakarot, otherwise false.
 */
export function isKakarotTransaction(transaction: Transaction): boolean {
  // Extract the 'to' address from the transaction's calldata, defaulting to 0 if undefined
  const toAddress = BigInt(transaction.invokeV1?.calldata?.[1] ?? 0);

  // Compare the 'to' address with KAKAROT_ADDRESS and return the result
  return toAddress === BigInt(KAKAROT_ADDRESS);
}

/**
 * Determines if Ethereum validation has failed based on event data.
 *
 * @param {Event} event - The event containing data to validate.
 * @returns {boolean} - Returns true if the validation failed, otherwise false.
 */
export function ethValidationFailed(event: Event): boolean {
  // Destructure to extract the 'data' array from the event
  const { data } = event;

  // Return false immediately if the 'data' array is empty
  if (!data.length) return false;

  // Parse the first element in 'data' as the response length, treating it as a hexadecimal string
  const responseLen = parseInt(data[0] ?? "", 16);

  // Validate that 'data' contains more elements than the response length + 1
  const isValidLength = data.length > responseLen + 1;

  // If 'data' length is invalid, log an error and return false
  if (!isValidLength) {
    console.error(
      `Invalid event data length. Got ${data.length}, expected > ${
        responseLen + 1
      }`,
    );
    return false;
  }

  // Parse the element at position (response length + 1) as the success flag
  const success = parseInt(data[responseLen + 1] ?? "", 16);

  // If the success flag is set to 1, return false as validation did not fail
  if (success === 1) return false;

  // Extract the response data slice, convert each byte to a character, and join them into a string
  const responseString = String.fromCharCode(
    ...data.slice(1, 1 + responseLen).map((x) => parseInt(x, 16)),
  );

  // Return true if the response string includes the phrase "eth validation failed"
  return responseString.includes("eth validation failed");
}

/**
 * Checks if a transaction receipt indicates a revert due to resource exhaustion.
 *
 * @param {TransactionReceipt} receipt - The transaction receipt to check.
 * @returns {boolean} - Returns true if the transaction was reverted due to running out of resources, otherwise false.
 */
export function isRevertedWithOutOfResources(
  receipt: TransactionReceipt,
): boolean {
  // Check if the execution status contains "REVERTED" and if the revert reason includes the specific error message
  return (
    receipt.executionStatus.includes("REVERTED") &&
    (receipt.revertReason ?? "").includes("RunResources has no remaining steps")
  );
}
