# web3_sha3
## Metadata
* name: web3_sha3
* prefix: web3
* state: ⚠️
* [specification](https://ethereum.org/en/developers/docs/apis/json-rpc/#web3_sha3)
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/5)
## Specification Description
Returns Keccak-256 (not the standardized SHA3-256) of the given data.
### Parameters
* DATA - bytes - the data to convert into a SHA3 hash
### Returns
* u256 - the data to convert into a SHA3 hash
## Kakarot Logic
This method does not interact with the kakarot contract or any other starknet contract. This method uses the `rust-crypto` package to hash the provided bytes with the keccak256 hash algorithm. The hex data provided was previously parsed by the `hex` utils.
### Kakarot methods
### Starknet methods