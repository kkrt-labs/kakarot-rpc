import { assertExists } from "https://deno.land/std@0.213.0/assert/mod.ts";
import {
  AccessListEIP2930Transaction,
  FeeMarketEIP1559Transaction,
  LegacyTransaction,
  RLP,
  Transaction,
  bytesToHex,
} from "../deps.ts";
import { toTypedEthTx } from "./transaction.ts";
import { assertEquals } from "https://deno.land/std@0.213.0/assert/assert_equals.ts";
import { Common } from "https://esm.sh/v135/@ethereumjs/common@4.1.0/denonext/common.mjs";

Deno.test("toTypedEthTx Legacy Transaction", () => {
  // Given
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new LegacyTransaction(
    {
      nonce: 1n,
      gasPrice: 2n,
      gasLimit: 3n,
      to: "0x0000000000000000000000000000000000000001",
      value: 4n,
      data: new Uint8Array([0x12, 0x34]),
    },
    { common },
  );
  const raw = RLP.encode(tx.getMessageToSign());

  const bytesLength = raw.byteLength;

  const serializedTx: `0x${string}`[] = [];
  for (let i = 0; i < raw.length; i += 31) {
    // byte chunk of 31 bytes
    const chunk = raw.slice(i, i + 31);
    // Convert to hex and push it to the serializedTx array
    serializedTx.push(bytesToHex(chunk) as `0x${string}`);
  }

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...serializedTx,
  ];

  const starknetTx: Transaction = {
    invokeV1: {
      senderAddress: "0x01",
      calldata: starknetTxCalldata,
    },
    meta: {
      hash: "0x01",
      maxFee: "0x01",
      nonce: "0x01",
      signature: ["0x1", "0x2", "0x3", "0x4", "0x32"],
      version: "1",
    },
  };

  // When
  const ethTx = toTypedEthTx({ transaction: starknetTx }) as LegacyTransaction;

  // Then
  assertExists(ethTx);
  assertEquals(ethTx.nonce, 1n);
  assertEquals(ethTx.gasPrice, 2n);
  assertEquals(ethTx.gasLimit, 3n);
  assertEquals(ethTx.value, 4n);
  assertEquals(ethTx.type, 0);
  assertEquals(ethTx.data, tx.data);
});

Deno.test("toTypedEthTx EIP1559 Transaction", () => {
  // Given
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new FeeMarketEIP1559Transaction(
    {
      nonce: 1n,
      maxFeePerGas: 4n,
      maxPriorityFeePerGas: 3n,
      gasLimit: 4n,
      to: "0x0000000000000000000000000000000000000001",
      value: 5n,
      data: new Uint8Array([0x12, 0x34]),
      accessList: [
        {
          address: "0x0000000000000000000000000000000000000002",
          storageKeys: [
            "0x0000000000000000000000000000000000000000000000000000000000000001",
          ],
        },
      ],
    },
    { common },
  );

  const raw = tx.getMessageToSign();
  const bytesLength = raw.byteLength;

  const serializedTx: `0x${string}`[] = [];
  for (let i = 0; i < raw.length; i += 31) {
    // byte chunk of 31 bytes
    const chunk = raw.slice(i, i + 31);
    // Convert to hex and push it to the serializedTx array
    serializedTx.push(bytesToHex(chunk) as `0x${string}`);
  }

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...serializedTx,
  ];

  const starknetTx: Transaction = {
    invokeV1: {
      senderAddress: "0x01",
      calldata: starknetTxCalldata,
    },
    meta: {
      hash: "0x01",
      maxFee: "0x01",
      nonce: "0x01",
      signature: ["0x1", "0x2", "0x3", "0x4", "0x1"],
      version: "1",
    },
  };

  // When
  const ethTx = toTypedEthTx({
    transaction: starknetTx,
  }) as FeeMarketEIP1559Transaction;

  // Then
  assertExists(ethTx);
  assertEquals(ethTx.nonce, 1n);
  assertEquals(ethTx.maxFeePerGas, 4n);
  assertEquals(ethTx.maxPriorityFeePerGas, 3n);
  assertEquals(ethTx.gasLimit, 4n);
  assertEquals(ethTx.value, 5n);
  assertEquals(ethTx.type, 2);
  assertEquals(ethTx.data, new Uint8Array([0x12, 0x34]));
});

Deno.test("toTypedEthTx EIP2930 Transaction", () => {
  // Given
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new AccessListEIP2930Transaction(
    {
      nonce: 1n,
      gasPrice: 2n,
      gasLimit: 3n,
      to: "0x0000000000000000000000000000000000000001",
      value: 4n,
      data: new Uint8Array([0x12, 0x34]),
      accessList: [
        {
          address: "0x0000000000000000000000000000000000000002",
          storageKeys: [
            "0x0000000000000000000000000000000000000000000000000000000000000001",
          ],
        },
      ],
    },
    { common },
  );

  const raw = tx.getMessageToSign();
  const bytesLength = raw.byteLength;

  const serializedTx: `0x${string}`[] = [];
  for (let i = 0; i < raw.length; i += 31) {
    // byte chunk of 31 bytes
    const chunk = raw.slice(i, i + 31);
    // Convert to hex and push it to the serializedTx array
    serializedTx.push(bytesToHex(chunk) as `0x${string}`);
  }

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...serializedTx,
  ];

  const starknetTx: Transaction = {
    invokeV1: {
      senderAddress: "0x01",
      calldata: starknetTxCalldata,
    },
    meta: {
      hash: "0x01",
      maxFee: "0x01",
      nonce: "0x01",
      signature: ["0x1", "0x2", "0x3", "0x4", "0x1"],
      version: "1",
    },
  };

  // When
  const ethTx = toTypedEthTx({
    transaction: starknetTx,
  }) as AccessListEIP2930Transaction;

  // Then
  // Then
  assertExists(ethTx);
  assertEquals(ethTx.nonce, 1n);
  assertEquals(ethTx.gasPrice, 2n);
  assertEquals(ethTx.gasLimit, 3n);
  assertEquals(ethTx.value, 4n);
  assertEquals(ethTx.type, 1);
  assertEquals(ethTx.data, tx.data);
  assertEquals(ethTx.accessList, tx.accessList);
});
