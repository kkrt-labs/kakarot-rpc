# kakarot_getTokenBalances

## Metadata

- name: kakarot_getTokenBalances
- prefix: kakarot
- state: ⚠️
- [specification](https://github.com/alchemyplatform/alchemy-web3#web3alchemygettokenbalancesaddress-contractaddresses)
- [issue](https://github.com/sayajin-labs/kakarot-rpc/issues/46)

## Specification Description

Returns token balances for a specific address given a list of contracts.

### Parameters

- address: The address for which token balances will be checked.
- contractAddress: An array of contract addresses.

### Returns

- TokenBalances with the following fields: 
  - address: The address for which token balances were checked.
  - tokenBalances: An array of token balance objects. Each object contains:
    - contractAddress: The address of the contract.
    - tokenBalance: The balance of the contract (bytes32)
    - error: An error string.

## Kakarot Logic

### Kakarot methods

- [execute_at_address] (https://sayajin-labs.github.io/kakarot-doc/docs/Kakarot/library#execute_at_address)
- [compute_starknet_address] (https://sayajin-labs.github.io/kakarot-doc/docs/Kakarot/Accounts/library#compute_starknet_address)

### Starknet methods

This method does not interact with Starknet RPC or any other Starknet contract.


### Example

