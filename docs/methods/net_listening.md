# net_listening
## Metadata
* name: net_listening
* prefix: net
* state: ⚠️ 🟡
* [specification](https://ethereum.org/en/developers/docs/apis/json-rpc/#net_listening)
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/8)
## Specification Description
Returns `true` if client is actively listening for network connections.
### Parameters
* None
### Returns
* boolean - `true` when listening, otherwise `false`.
## Kakarot Logic
For the moment, Kakarot is not a layer 3 so not a dedicated network. The RPC is not connected to any specific network with other nodes except to starknet. So as a temporary solution it returns `true` if the connection is successful with the starknet RPC and if not it returns `false`.
### Kakarot methods
### Starknet methods