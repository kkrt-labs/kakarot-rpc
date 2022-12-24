# web3_clientVersion
## Metadata
* name: web3_clientVersion
* prefix: web3
* state: ⚠️
* [specification](https://ethereum.org/en/developers/docs/apis/json-rpc/#web3_clientversion)
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/4)
## Specification Description
Returns the current client version.
### Parameters
* None
### Returns
* String - Version of the client
## Kakarot Logic
This method does not interact with the kakarot contract or any other starknet contract. Instead it will return the version of this repository with this format:
> `kakarot-rpc-adapter/vX.X.X/OS/rustX.X.X`
### Kakarot methods
### Starknet methods