# net_version
## Metadata
* name: net_version
* prefix: net
* state: ⚠️
* [specification](https://ethereum.org/en/developers/docs/apis/json-rpc/#net_version)
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/6)
## Specification Description
Returns the current network id.
### Parameters
* None
### Returns
* String - the current network id
## Kakarot Logic
This method does not interact with the kakarot contract or any other starknet contract. The method returns a constant variable name `CHAIN_ID` which needs to be determined see this [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/2).
### Kakarot methods
### Starknet methods