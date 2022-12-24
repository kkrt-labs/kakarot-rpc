# kakarot-rpc-adapter
Kakarot ZK EVM Ethereum RPC adapter

## JSON-RPC API Methods
### web3
* [web3_clientVersion](https://ethereum.org/en/developers/docs/apis/json-rpc/#web3_clientversion)
* [web3_sha3](https://ethereum.org/en/developers/docs/apis/json-rpc/#web3_sha3)
### net
* [net_version](https://ethereum.org/en/developers/docs/apis/json-rpc/#net_version)
* [net_listening](https://ethereum.org/en/developers/docs/apis/json-rpc/#net_listening)
* [net_peerCount](https://ethereum.org/en/developers/docs/apis/json-rpc/#net_listening)
### eth
* TODO 🚧

## Crate structure
```
crate kakarot-rpc-adapter
- module utils 
   - module hex
   - module error
   - module constants
- module methods 
  - module web3
     - // a module for each method with a METHOD const and execute function
  - module net
      - // a module for each method with a METHOD const and execute function
  - module eth
      - // a module for each method with a METHOD const and execute function
```
