# {METHOD_NAME}
## Metadata
* name: {method_name}
* prefix: {"eth" | "web3" | "net"}
* state: {❌ | ⚠️ |⏳ | ✅ |🟡}
* [specification](https://ethereum.org/en/developers/docs/apis/json-rpc/#{method_name})
* [issue](https://github.com/sayajin-labs/kakarot-rpc-adapter/issues/{issue_id})
## Specification Description
The method behaviour following the specification (copy & paste authorized)
### Parameters
* Name - type - brief description (see [types](types.md))
### Returns
* type - brief description (see [types](types.md))
## Kakarot Logic
How is the method working with the kakarot and starknet solution
### Kakarot methods
The Kakarot contract methods needed listed and linked to the github [repo](https://github.com/sayajin-labs/kakarot/blob/56cb71852e61b755eeeb5895f763357fce62b4d5/src/kakarot) line with perma link
* [kakarot.cairo::execute_at_address](https://github.com/sayajin-labs/kakarot/blob/56cb71852e61b755eeeb5895f763357fce62b4d5/src/kakarot/kakarot.cairo#L86)
### Starknet methods
The Starknet api methods needed listed and linked to the [starknet api reference file](https://github.com/starkware-libs/starknet-specs/blob/63bdb0fe3e7c0fd21bc47b2301528bff32980bf6/api/starknet_api_openrpc.json) and line perma link
* [starknet_getBlockWithTxHashes](https://github.com/starkware-libs/starknet-specs/blob/63bdb0fe3e7c0fd21bc47b2301528bff32980bf6/api/starknet_api_openrpc.json#L11)