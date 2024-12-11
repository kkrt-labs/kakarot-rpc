# eth_gasPrice

## Metadata

- name: eth_gasPrice
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/fee_market.yaml#L1)

## Description

Returns the current price per gas in wei.

Kakarot specifity: since Kakarot does not have a fee market as of January 2024,
transactions are ordered on a "First Come First Serve" basis.

For this reason:

- gasPrice == baseFee
- priority fee is generally a variable that isn't used.
  - setting an EIP-1559 transaction with `maxPriorityFeePerGas > 0` has no
    effect.
