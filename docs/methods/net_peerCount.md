# net_peerCount
## Metadata
* name: net_peerCount
* prefix: net
* state: ⚠️ 🟡
* [specification](https://ethereum.org/en/developers/docs/apis/json-rpc/#net_peercount)
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/9)
## Specification Description
Returns the number of peers currently connected to the client.
### Parameters
* None
### Returns
* Number - unsigned integer of the number of connected peers.
## Kakarot Logic
For the moment, Kakarot is not a layer 3 so not a dedicated network. The RPC is not connected to any specific network with other nodes except to starknet. So as a temporary solution it returns `0x1` if the connection is successful with the starknet RPC and if not it returns `0x0`.
### Kakarot methods
### Starknet methods