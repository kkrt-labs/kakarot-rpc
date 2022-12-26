# eth_chainId
## Metadata
* name: eth_chainId
* prefix: eth
* state: ⚠️
* [specification](https://github.com/ethereum/execution-apis/blob/6709c2a795b707202e93c4f2867fa0bf2640a84f/src/eth/client.yaml#L1)
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/7)
## Specification Description
Returns the chain ID of the current network.
### Parameters
* None
### Returns
* uint - chainId
## Kakarot Logic
This method does not interact with the kakarot contract or any other starknet contract. The method returns a constant variable name `CHAIN_ID` which needs to be determined see this [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/2).
### Kakarot methods
### Starknet methods