# eth_sendRawTransaction

## Metadata

- name: sendRawTransaction
- prefix: eth
- state: ⚠️
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/submit.yaml)
- [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/22)

## Specification Description

Sends a raw transaction (RLP Encoded) to be submitted to the network.

### Parameters

- string - transaction

### Returns

- bytes32 - transactionHash

## Kakarot Logic

This method does not interact with the Kakarot contract directly.
It calls the Starknet sequencer => Starknet sequencer calls EOA account => EOA account calls validate and then execute.


