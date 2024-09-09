import { assertExists } from "https://deno.land/std@0.213.0/assert/mod.ts";
import {
  AccessListEIP2930Transaction,
  FeeMarketEIP1559Transaction,
  hexToBytes,
  JsonTx,
  LegacyTransaction,
  RLP,
  Transaction,
  TransactionReceipt,
  TransactionWithReceipt,
} from "../deps.ts";
import {
  chainId,
  packCallData,
  setFlagRunOutOfResources,
  setYParityFlag,
  toEthTx,
  toTypedEthTx,
  transactionEthFormat,
  typedTransactionToEthTx,
  unpackCallData,
} from "./transaction.ts";
import { assertEquals } from "https://deno.land/std@0.213.0/assert/assert_equals.ts";
import { Common } from "https://esm.sh/v135/@ethereumjs/common@4.1.0/denonext/common.mjs";
import { ExtendedJsonRpcTx } from "./interfaces.ts";
import {
  EXPECTED_TRANSFORM_DATA_FILE,
  TRANSACTIONS_DATA_FILE,
} from "../testConstants.ts";

// Transaction data including headers, events, and transactions
const jsonTransactionsData = await Deno.readTextFile(
  TRANSACTIONS_DATA_FILE,
);
const transactionsData = JSON.parse(jsonTransactionsData);

// Expected output after transform and toTypedEthTx transformation for comparison in tests
const jsonExpectedTransformData = await Deno.readTextFile(
  EXPECTED_TRANSFORM_DATA_FILE,
);
const expectedTransformData = JSON.parse(jsonExpectedTransformData);

// Utility functions
function createReceipt(
  overrides: Partial<TransactionReceipt> = {},
): TransactionReceipt {
  return {
    executionStatus: "EXECUTION_STATUS_SUCCEEDED",
    transactionHash: "0x456",
    transactionIndex: "0x1",
    actualFee: "0x1",
    contractAddress: "0xabc",
    l2ToL1Messages: [],
    events: [],
    ...overrides,
  };
}

function createSignedLegacyTransaction(): LegacyTransaction {
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const privateKeyHex =
    "4c0883a69102937d6234140ed2a7213e592d98b700b97d9a7325c3b3b7fafa90"; // Example private key, replace with your own
  const privateKey = new Uint8Array(
    privateKeyHex.match(/.{1,2}/g)!.map((byte) => parseInt(byte, 16)),
  ); // Convert hex string to Uint8Array

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

  const signedTx = tx.sign(privateKey);
  return signedTx;
}

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

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...packCallData(raw),
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

Deno.test("toTypedEthTx Legacy Transaction with v = 28", () => {
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

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...packCallData(raw),
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
      signature: ["0x1", "0x2", "0x3", "0x4", "0x1c"], // 0x1c -> 28
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

Deno.test("toTypedEthTx Legacy Transaction with v = 26 (failure case)", () => {
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

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...packCallData(raw),
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
      signature: ["0x1", "0x2", "0x3", "0x4", "0x1a"], // 0x1a -> 26
      version: "1",
    },
  };

  // When
  const ethTx = toTypedEthTx({ transaction: starknetTx }) as LegacyTransaction;

  // Then
  assertEquals(ethTx, null);
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

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...packCallData(raw),
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

  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    `0x${bytesLength.toString(16)}`,
    ...packCallData(raw),
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
  assertExists(ethTx);
  assertEquals(ethTx.nonce, 1n);
  assertEquals(ethTx.gasPrice, 2n);
  assertEquals(ethTx.gasLimit, 3n);
  assertEquals(ethTx.value, 4n);
  assertEquals(ethTx.type, 1);
  assertEquals(ethTx.data, tx.data);
  assertEquals(ethTx.accessList, tx.accessList);
});

Deno.test("packCallData by chunks", () => {
  const input = hexToBytes(
    "0x01f904fb846b6b7274218083049e0e8080b904ea608060405260405161040a38038061040a83398101604081905261002291610268565b61002c8282610033565b5050610352565b61003c82610092565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b90600090a280511561008657610081828261010e565b505050565b61008e610185565b5050565b806001600160a01b03163b6000036100cd57604051634c9c8ce360e01b81526001600160a01b03821660048201526024015b60405180910390fd5b7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc80546001600160a01b0319166001600160a01b0392909216919091179055565b6060600080846001600160a01b03168460405161012b9190610336565b600060405180830381855af49150503d8060008114610166576040519150601f19603f3d011682016040523d82523d6000602084013e61016b565b606091505b50909250905061017c8583836101a6565b95945050505050565b34156101a45760405163b398979f60e01b815260040160405180910390fd5b565b6060826101bb576101b682610205565b6101fe565b81511580156101d257506001600160a01b0384163b155b156101fb57604051639996b31560e01b81526001600160a01b03851660048201526024016100c4565b50805b9392505050565b8051156102155780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b634e487b7160e01b600052604160045260246000fd5b60005b8381101561025f578181015183820152602001610247565b50506000910152565b6000806040838503121561027b57600080fd5b82516001600160a01b038116811461029257600080fd5b60208401519092506001600160401b03808211156102af57600080fd5b818501915085601f8301126102c357600080fd5b8151818111156102d5576102d561022e565b604051601f8201601f19908116603f011681019083821181831017156102fd576102fd61022e565b8160405282815288602084870101111561031657600080fd5b610327836020830160208801610244565b80955050505050509250929050565b60008251610348818460208701610244565b9190910192915050565b60aa806103606000396000f3fe6080604052600a600c565b005b60186014601a565b6051565b565b6000604c7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b905090565b3660008037600080366000845af43d6000803e808015606f573d6000f35b3d6000fdfea2646970667358221220d0232cfa81216c3e4973e570f043b57ccb69ae4a81b8bc064338713721c87a9f64736f6c6343000814003300000000000000000000000009635f643e140090a9a8dcd712ed6285858cebef000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000647a1ac61e00000000000000000000000084ea74d481ee0a5332c457a4d796187f6ba67feb00000000000000000000000000000000000000000000000000038d7ea4c68000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000c0",
  );

  const expected_packed_data: `0x${string}`[] = [
    "0x0001f904fb846b6b7274218083049e0e8080b904ea608060405260405161040a",
    "0x0038038061040a83398101604081905261002291610268565b61002c82826100",
    "0x0033565b5050610352565b61003c82610092565b6040516001600160a01b0383",
    "0x0016907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da",
    "0x002e5c2d3b90600090a280511561008657610081828261010e565b505050565b",
    "0x0061008e610185565b5050565b806001600160a01b03163b6000036100cd5760",
    "0x004051634c9c8ce360e01b81526001600160a01b03821660048201526024015b",
    "0x0060405180910390fd5b7f360894a13ba1a3210667c828492db98dca3e2076cc",
    "0x003735a920a3ca505d382bbc80546001600160a01b0319166001600160a01b03",
    "0x0092909216919091179055565b6060600080846001600160a01b031684604051",
    "0x0061012b9190610336565b600060405180830381855af49150503d8060008114",
    "0x00610166576040519150601f19603f3d011682016040523d82523d6000602084",
    "0x00013e61016b565b606091505b50909250905061017c8583836101a6565b9594",
    "0x005050505050565b34156101a45760405163b398979f60e01b81526004016040",
    "0x005180910390fd5b565b6060826101bb576101b682610205565b6101fe565b81",
    "0x00511580156101d257506001600160a01b0384163b155b156101fb5760405163",
    "0x009996b31560e01b81526001600160a01b03851660048201526024016100c456",
    "0x005b50805b9392505050565b8051156102155780518082602001fd5b60405163",
    "0x000a12f52160e11b815260040160405180910390fd5b634e487b7160e01b6000",
    "0x0052604160045260246000fd5b60005b8381101561025f578181015183820152",
    "0x00602001610247565b50506000910152565b6000806040838503121561027b57",
    "0x00600080fd5b82516001600160a01b038116811461029257600080fd5b602084",
    "0x0001519092506001600160401b03808211156102af57600080fd5b8185019150",
    "0x0085601f8301126102c357600080fd5b8151818111156102d5576102d561022e",
    "0x00565b604051601f8201601f19908116603f0116810190838211818310171561",
    "0x0002fd576102fd61022e565b8160405282815288602084870101111561031657",
    "0x00600080fd5b610327836020830160208801610244565b809550505050505092",
    "0x0050929050565b60008251610348818460208701610244565b91909101929150",
    "0x0050565b60aa806103606000396000f3fe6080604052600a600c565b005b6018",
    "0x006014601a565b6051565b565b6000604c7f360894a13ba1a3210667c828492d",
    "0x00b98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b",
    "0x00905090565b3660008037600080366000845af43d6000803e808015606f573d",
    "0x006000f35b3d6000fdfea2646970667358221220d0232cfa81216c3e4973e570",
    "0x00f043b57ccb69ae4a81b8bc064338713721c87a9f64736f6c63430008140033",
    "0x0000000000000000000000000009635f643e140090a9a8dcd712ed6285858ceb",
    "0x00ef000000000000000000000000000000000000000000000000000000000000",
    "0x0000400000000000000000000000000000000000000000000000000000000000",
    "0x000000647a1ac61e00000000000000000000000084ea74d481ee0a5332c457a4",
    "0x00d796187f6ba67feb0000000000000000000000000000000000000000000000",
    "0x000000038d7ea4c6800000000000000000000000000000000000000000000000",
    "0x0000000000000000000014000000000000000000000000000000000000000000",
    "0x00000000000000000000000000000000000000000000000000000000000000c0",
  ];

  assertEquals(packCallData(input), expected_packed_data);
});

Deno.test("unpackCallData by chunks", () => {
  const input: `0x${string}`[] = [
    "0x0000000000000000000000000000000000000000000000000000000000000001",
    "0x07a4394ca8608f89bb947e81d364e18a3651369c67ec1f008ae2b9fae3dee69a",
    "0x007099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
    "0x0000000000000000000000000000000000000000000000000000000000000000",
    "0x000000000000000000000000000000000000000000000000000000000000002b",
    "0x000000000000000000000000000000000000000000000000000000000000002b",
    "0x00000000000000000000000000000000000000000000000000000000000004ff",
    "0x0001f904fb846b6b7274218083049e0e8080b904ea608060405260405161040a",
    "0x0038038061040a83398101604081905261002291610268565b61002c82826100",
    "0x0033565b5050610352565b61003c82610092565b6040516001600160a01b0383",
    "0x0016907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da",
    "0x002e5c2d3b90600090a280511561008657610081828261010e565b505050565b",
    "0x0061008e610185565b5050565b806001600160a01b03163b6000036100cd5760",
    "0x004051634c9c8ce360e01b81526001600160a01b03821660048201526024015b",
    "0x0060405180910390fd5b7f360894a13ba1a3210667c828492db98dca3e2076cc",
    "0x003735a920a3ca505d382bbc80546001600160a01b0319166001600160a01b03",
    "0x0092909216919091179055565b6060600080846001600160a01b031684604051",
    "0x0061012b9190610336565b600060405180830381855af49150503d8060008114",
    "0x00610166576040519150601f19603f3d011682016040523d82523d6000602084",
    "0x00013e61016b565b606091505b50909250905061017c8583836101a6565b9594",
    "0x005050505050565b34156101a45760405163b398979f60e01b81526004016040",
    "0x005180910390fd5b565b6060826101bb576101b682610205565b6101fe565b81",
    "0x00511580156101d257506001600160a01b0384163b155b156101fb5760405163",
    "0x009996b31560e01b81526001600160a01b03851660048201526024016100c456",
    "0x005b50805b9392505050565b8051156102155780518082602001fd5b60405163",
    "0x000a12f52160e11b815260040160405180910390fd5b634e487b7160e01b6000",
    "0x0052604160045260246000fd5b60005b8381101561025f578181015183820152",
    "0x00602001610247565b50506000910152565b6000806040838503121561027b57",
    "0x00600080fd5b82516001600160a01b038116811461029257600080fd5b602084",
    "0x0001519092506001600160401b03808211156102af57600080fd5b8185019150",
    "0x0085601f8301126102c357600080fd5b8151818111156102d5576102d561022e",
    "0x00565b604051601f8201601f19908116603f0116810190838211818310171561",
    "0x0002fd576102fd61022e565b8160405282815288602084870101111561031657",
    "0x00600080fd5b610327836020830160208801610244565b809550505050505092",
    "0x0050929050565b60008251610348818460208701610244565b91909101929150",
    "0x0050565b60aa806103606000396000f3fe6080604052600a600c565b005b6018",
    "0x006014601a565b6051565b565b6000604c7f360894a13ba1a3210667c828492d",
    "0x00b98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b",
    "0x00905090565b3660008037600080366000845af43d6000803e808015606f573d",
    "0x006000f35b3d6000fdfea2646970667358221220d0232cfa81216c3e4973e570",
    "0x00f043b57ccb69ae4a81b8bc064338713721c87a9f64736f6c63430008140033",
    "0x0000000000000000000000000009635f643e140090a9a8dcd712ed6285858ceb",
    "0x00ef000000000000000000000000000000000000000000000000000000000000",
    "0x0000400000000000000000000000000000000000000000000000000000000000",
    "0x000000647a1ac61e00000000000000000000000084ea74d481ee0a5332c457a4",
    "0x00d796187f6ba67feb0000000000000000000000000000000000000000000000",
    "0x000000038d7ea4c6800000000000000000000000000000000000000000000000",
    "0x0000000000000000000014000000000000000000000000000000000000000000",
    "0x00000000000000000000000000000000000000000000000000000000000000c0",
  ];

  const expected_calldata = hexToBytes(
    "0x01f904fb846b6b7274218083049e0e8080b904ea608060405260405161040a38038061040a83398101604081905261002291610268565b61002c8282610033565b5050610352565b61003c82610092565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b90600090a280511561008657610081828261010e565b505050565b61008e610185565b5050565b806001600160a01b03163b6000036100cd57604051634c9c8ce360e01b81526001600160a01b03821660048201526024015b60405180910390fd5b7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc80546001600160a01b0319166001600160a01b0392909216919091179055565b6060600080846001600160a01b03168460405161012b9190610336565b600060405180830381855af49150503d8060008114610166576040519150601f19603f3d011682016040523d82523d6000602084013e61016b565b606091505b50909250905061017c8583836101a6565b95945050505050565b34156101a45760405163b398979f60e01b815260040160405180910390fd5b565b6060826101bb576101b682610205565b6101fe565b81511580156101d257506001600160a01b0384163b155b156101fb57604051639996b31560e01b81526001600160a01b03851660048201526024016100c4565b50805b9392505050565b8051156102155780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b634e487b7160e01b600052604160045260246000fd5b60005b8381101561025f578181015183820152602001610247565b50506000910152565b6000806040838503121561027b57600080fd5b82516001600160a01b038116811461029257600080fd5b60208401519092506001600160401b03808211156102af57600080fd5b818501915085601f8301126102c357600080fd5b8151818111156102d5576102d561022e565b604051601f8201601f19908116603f011681019083821181831017156102fd576102fd61022e565b8160405282815288602084870101111561031657600080fd5b610327836020830160208801610244565b80955050505050509250929050565b60008251610348818460208701610244565b9190910192915050565b60aa806103606000396000f3fe6080604052600a600c565b005b60186014601a565b6051565b565b6000604c7f360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc546001600160a01b031690565b905090565b3660008037600080366000845af43d6000803e808015606f573d6000f35b3d6000fdfea2646970667358221220d0232cfa81216c3e4973e570f043b57ccb69ae4a81b8bc064338713721c87a9f64736f6c6343000814003300000000000000000000000009635f643e140090a9a8dcd712ed6285858cebef000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000647a1ac61e00000000000000000000000084ea74d481ee0a5332c457a4d796187f6ba67feb00000000000000000000000000000000000000000000000000038d7ea4c68000000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000c0",
  );

  assertEquals(unpackCallData(input), expected_calldata);
});

Deno.test("toTypedEthTx Legacy Transaction before release with 31 bytes chunks packing", () => {
  // Given
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new LegacyTransaction({
    nonce: 1n,
    gasPrice: 2n,
    gasLimit: 3n,
    to: "0x0000000000000000000000000000000000000001",
    value: 4n,
    data: new Uint8Array([0x12, 0x34]),
  }, { common });
  const raw = RLP.encode(tx.getMessageToSign());

  const serializedTx: `0x${string}`[] = [];
  raw.forEach((x) => serializedTx.push(`0x${x.toString(16)}`));
  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
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

Deno.test("toTypedEthTx EIP1559 Transaction before release with 31 bytes chunks packing", () => {
  // Given
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new FeeMarketEIP1559Transaction({
    nonce: 1n,
    maxFeePerGas: 4n,
    maxPriorityFeePerGas: 3n,
    gasLimit: 4n,
    to: "0x0000000000000000000000000000000000000001",
    value: 5n,
    data: new Uint8Array([0x12, 0x34]),
    accessList: [{
      address: "0x0000000000000000000000000000000000000002",
      storageKeys: [
        "0x0000000000000000000000000000000000000000000000000000000000000001",
      ],
    }],
  }, { common });

  const raw = tx.getMessageToSign();
  const serializedTx: `0x${string}`[] = [];
  raw.forEach((x) => serializedTx.push(`0x${x.toString(16)}`));
  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
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

Deno.test("toTypedEthTx EIP2930 Transaction before release with 31 bytes chunks packing", () => {
  // Given
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new AccessListEIP2930Transaction({
    nonce: 1n,
    gasPrice: 2n,
    gasLimit: 3n,
    to: "0x0000000000000000000000000000000000000001",
    value: 4n,
    data: new Uint8Array([0x12, 0x34]),
    accessList: [{
      address: "0x0000000000000000000000000000000000000002",
      storageKeys: [
        "0x0000000000000000000000000000000000000000000000000000000000000001",
      ],
    }],
  }, { common });

  const raw = tx.getMessageToSign();
  const serializedTx: `0x${string}`[] = [];
  raw.forEach((x) => serializedTx.push(`0x${x.toString(16)}`));
  const starknetTxCalldata: `0x${string}`[] = [
    "0x1",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
    "0x0",
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
  assertExists(ethTx);
  assertEquals(ethTx.nonce, 1n);
  assertEquals(ethTx.gasPrice, 2n);
  assertEquals(ethTx.gasLimit, 3n);
  assertEquals(ethTx.value, 4n);
  assertEquals(ethTx.type, 1);
  assertEquals(ethTx.data, tx.data);
  assertEquals(ethTx.accessList, tx.accessList);
});

Deno.test("toTypedEthTx with real data", () => {
  transactionsData.transactionsList.forEach(
    (transactions: TransactionWithReceipt[], outerIndex: number) => {
      transactions.map((transaction, innerIndex) => {
        const ethTx = toTypedEthTx({
          transaction: transaction.transaction,
        });
        assertEquals(
          JSON.stringify(ethTx),
          JSON.stringify(
            expectedTransformData
              .expectedToTypedEthTxTransactions[outerIndex][innerIndex],
          ),
        );
      });
    },
  );
});

Deno.test("toEthTx returns null for invalid transaction", () => {
  const result = toEthTx({
    transaction: {} as Transaction,
    receipt: {} as TransactionReceipt,
    blockNumber: "0x1",
    blockHash: "0x123",
    isPendingBlock: false,
  });
  assertEquals(result, null);
});

Deno.test("chainId calculates chain ID from v value for legacy transaction", () => {
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
  const jsonTx = tx.toJSON();
  jsonTx.v = "0x25"; // 37 in decimal, which corresponds to chain ID 1 (mainnet)

  const result = chainId(tx, jsonTx);
  assertEquals(result, "0x1");
});

Deno.test("chainId returns chainId directly when isLegacyTx is false", () => {
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
      accessList: [],
    },
    { common },
  );
  const jsonTx = { chainId: "0x1" } as JsonTx;

  const result = chainId(tx, jsonTx);
  assertEquals(result, "0x1");
});

Deno.test("chainId returns undefined when jsonTx.v is undefined", () => {
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
  const jsonTx = { v: undefined } as JsonTx;

  // When
  const id = chainId(tx, jsonTx);

  // Then
  assertEquals(id, undefined);
});

Deno.test("setYParityFlag adds yParity field for EIP1559 transaction", () => {
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
      accessList: [],
    },
    { common },
  );
  const jsonTx = tx.toJSON();
  jsonTx.v = "0x1"; // Adding v value for yParity

  const result: ExtendedJsonRpcTx = {} as ExtendedJsonRpcTx;
  setYParityFlag(tx, jsonTx, result);
  assertEquals(result.yParity, "0x1");
});

Deno.test("setYParityFlag adds yParity field for EIP2930 transaction", () => {
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
  const jsonTx = tx.toJSON();
  jsonTx.v = "0x1"; // Adding v value for yParity

  const result: ExtendedJsonRpcTx = {} as ExtendedJsonRpcTx;
  setYParityFlag(tx, jsonTx, result);
  assertEquals(result.yParity, "0x1");
});

Deno.test("setYParityFlag does not add yParity field for legacy transaction", () => {
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
  const jsonTx = tx.toJSON();
  jsonTx.v = "0x1";

  const result: ExtendedJsonRpcTx = {} as ExtendedJsonRpcTx;
  setYParityFlag(tx, jsonTx, result);
  assertEquals(result.yParity, undefined);
});

Deno.test("setFlagRunOutOfResources does not add isRunOutOfResources flag for successful transaction", () => {
  const receipt = createReceipt({
    executionStatus: "EXECUTION_STATUS_SUCCEEDED",
  });
  const result: ExtendedJsonRpcTx = {} as ExtendedJsonRpcTx;
  setFlagRunOutOfResources(receipt, result);
  assertEquals(result.isRunOutOfResources, undefined);
});

Deno.test("setFlagRunOutOfResources adds isRunOutOfResources flag for out of resources transaction", () => {
  const receipt = createReceipt({
    executionStatus: "EXECUTION_STATUS_REVERTED",
    revertReason: "RunResources has no remaining steps",
  }); // Indicating out of resources
  const result: ExtendedJsonRpcTx = {
    isRunOutOfResources: false,
  } as ExtendedJsonRpcTx;

  setFlagRunOutOfResources(receipt, result);
  assertEquals(result.isRunOutOfResources, true);
});

Deno.test("typedTransactionToEthTx returns null for unsigned transaction", () => {
  const common = new Common({ chain: "mainnet", hardfork: "shanghai" });
  const tx = new LegacyTransaction({
    nonce: 1n,
    gasPrice: 2n,
    gasLimit: 3n,
    to: "0x0000000000000000000000000000000000000001",
    value: 4n,
    data: new Uint8Array([0x12, 0x34]),
  }, { common });

  const ethTx = typedTransactionToEthTx({
    typedTransaction: tx,
    receipt: {} as TransactionReceipt,
    blockNumber: "0x1",
    blockHash: "0x123",
    isPendingBlock: false,
  });

  assertEquals(ethTx, null);
});

Deno.test("typedTransactionToEthTx handles missing transaction index", () => {
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
  const receipt = createReceipt({ transactionIndex: undefined });
  const result = typedTransactionToEthTx({
    typedTransaction: tx,
    receipt,
    blockNumber: "0x1",
    blockHash: "0x123",
    isPendingBlock: false,
  });
  assertEquals(result, null);
});

Deno.test("typedTransactionToEthTx handles legacy transaction", () => {
  const tx = createSignedLegacyTransaction();
  const jsonTx = tx.toJSON();

  const result = transactionEthFormat({
    typedTransaction: tx,
    jsonTx,
    receipt: { transactionIndex: "0x1" } as TransactionReceipt,
    blockNumber: "0x1",
    blockHash: "0x123",
    isPendingBlock: false,
    chainId: "0x1",
    index: "0x1",
  });

  assertExists(result);
  assertEquals(result?.from, tx.getSenderAddress().toString());
  assertEquals(result?.blockHash, "0x123");
  assertEquals(result?.blockNumber, "0x1");
  assertEquals(result?.gas, jsonTx.gasLimit);
  assertEquals(result?.gasPrice, jsonTx.gasPrice);
  assertEquals(result?.chainId, "0x1");
  assertEquals(result?.input, jsonTx.data);
  assertEquals(result?.nonce, jsonTx.nonce);
  assertEquals(result?.to, tx.to?.toString());
  assertEquals(result?.value, jsonTx.value);
  assertEquals(result?.v, jsonTx.v);
  assertEquals(result?.r, jsonTx.r);
  assertEquals(result?.s, jsonTx.s);
});
