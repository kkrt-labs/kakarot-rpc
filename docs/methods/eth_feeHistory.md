# eth_feeHistory

## Metadata

- name: eth_feeHistory
- prefix: eth
- state: ✅
- [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/fee_market.yaml#L17)

## Description

Returns transaction base fee per gas and effective priority fee per gas for the requested/supported block range.

Kakarot Specificity:

- Since Kakarot has no fee market, this will return the base fee over a range of blocks (since priority fee is always null, we get `gasPrice == baseFee` all the time).
- The reward percentile logic does not apply, and the gasUsed ratio is hardcoded to 1.

Note:

- Using this endpoint is discouraged and is made somewhat compatible to avoid breaking existing backend logic.
