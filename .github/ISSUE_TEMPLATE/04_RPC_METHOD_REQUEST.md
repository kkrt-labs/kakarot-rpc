---
name: Add a New RPC Method
about:
  Provide information about an RPC method to be added to the Kakarot RPC server.
title: "feat: "
labels: "new-feature"
assignees: ""
---

# <METHOD_NAME>

## Metadata

- name:
- prefix:
- state:
- [specification](https://github.com/ethereum/execution-apis/<SPEC_URL>)

## Specification Description

Describe the method

### Parameters

- None

### Returns

- uint64 - e.g. blockNumber

## Kakarot Logic

Describe the interaction with Kakarot and/or Starknet. e.g. "this method does
not interact with the kakarot contract or any other starknet contract."

### Kakarot methods

Which Kakarot methods are called?

### Starknet methods

Which Starknet RPC methods are called? e.g. "Would be calling the corresponding
`starknet_blockNumber`
https://github.com/starkware-libs/starknet-specs/blob/master/starknet_vs_ethereum_node_apis.md"
