import { Contract, RpcProvider } from "./deps.ts";
import { KAKAROT_ADDRESS } from "./constants.ts";

// Get the URL of the Starknet Network
const RPC_URL = (() => {
  const rpcUrl = Deno.env.get("STARKNET_NETWORK");
  if (!rpcUrl) throw new Error("ENV: STARKNET_NETWORK is not set");
  return rpcUrl;
})();

// Get the RPC Provider from the Starknet Network
export const PROVIDER = new RpcProvider({
  nodeUrl: RPC_URL,
});

const abi = [
  {
    members: [
      {
        name: "low",
        offset: 0,
        type: "felt",
      },
      {
        name: "high",
        offset: 1,
        type: "felt",
      },
    ],
    name: "Uint256",
    size: 2,
    type: "struct",
  },
  {
    members: [
      {
        name: "is_some",
        offset: 0,
        type: "felt",
      },
      {
        name: "value",
        offset: 1,
        type: "felt",
      },
    ],
    name: "Option",
    size: 2,
    type: "struct",
  },
  {
    data: [
      {
        name: "previousOwner",
        type: "felt",
      },
      {
        name: "newOwner",
        type: "felt",
      },
    ],
    keys: [],
    name: "OwnershipTransferred",
    type: "event",
  },
  {
    data: [
      {
        name: "evm_contract_address",
        type: "felt",
      },
      {
        name: "starknet_contract_address",
        type: "felt",
      },
    ],
    keys: [],
    name: "evm_contract_deployed",
    type: "event",
  },
  {
    data: [
      {
        name: "new_class_hash",
        type: "felt",
      },
    ],
    keys: [],
    name: "kakarot_upgraded",
    type: "event",
  },
  {
    inputs: [
      {
        name: "owner",
        type: "felt",
      },
      {
        name: "native_token_address",
        type: "felt",
      },
      {
        name: "account_contract_class_hash",
        type: "felt",
      },
      {
        name: "uninitialized_account_class_hash",
        type: "felt",
      },
      {
        name: "cairo1_helpers_class_hash",
        type: "felt",
      },
      {
        name: "coinbase",
        type: "felt",
      },
      {
        name: "block_gas_limit",
        type: "felt",
      },
    ],
    name: "constructor",
    outputs: [],
    type: "constructor",
  },
  {
    inputs: [
      {
        name: "new_class_hash",
        type: "felt",
      },
    ],
    name: "upgrade",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_owner",
    outputs: [
      {
        name: "owner",
        type: "felt",
      },
    ],
    type: "function",
  },
  {
    inputs: [
      {
        name: "new_owner",
        type: "felt",
      },
    ],
    name: "transfer_ownership",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "native_token_address",
        type: "felt",
      },
    ],
    name: "set_native_token",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_native_token",
    outputs: [
      {
        name: "native_token_address",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "base_fee",
        type: "felt",
      },
    ],
    name: "set_base_fee",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_base_fee",
    outputs: [
      {
        name: "base_fee",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "coinbase",
        type: "felt",
      },
    ],
    name: "set_coinbase",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_coinbase",
    outputs: [
      {
        name: "coinbase",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "prev_randao",
        type: "Uint256",
      },
    ],
    name: "set_prev_randao",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_prev_randao",
    outputs: [
      {
        name: "prev_randao",
        type: "Uint256",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "gas_limit_",
        type: "felt",
      },
    ],
    name: "set_block_gas_limit",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_block_gas_limit",
    outputs: [
      {
        name: "block_gas_limit",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
    ],
    name: "compute_starknet_address",
    outputs: [
      {
        name: "contract_address",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [],
    name: "get_account_contract_class_hash",
    outputs: [
      {
        name: "account_contract_class_hash",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "account_contract_class_hash",
        type: "felt",
      },
    ],
    name: "set_account_contract_class_hash",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
      {
        name: "authorized",
        type: "felt",
      },
    ],
    name: "set_authorized_cairo_precompile_caller",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "cairo1_helpers_class_hash",
        type: "felt",
      },
    ],
    name: "set_cairo1_helpers_class_hash",
    outputs: [],
    type: "function",
  },
  {
    inputs: [],
    name: "get_cairo1_helpers_class_hash",
    outputs: [
      {
        name: "cairo1_helpers_class_hash",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
    ],
    name: "get_starknet_address",
    outputs: [
      {
        name: "starknet_address",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
    ],
    name: "deploy_externally_owned_account",
    outputs: [
      {
        name: "starknet_contract_address",
        type: "felt",
      },
    ],
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
    ],
    name: "register_account",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
      {
        name: "bytecode_len",
        type: "felt",
      },
      {
        name: "bytecode",
        type: "felt*",
      },
    ],
    name: "write_account_bytecode",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
    ],
    name: "upgrade_account",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "evm_address",
        type: "felt",
      },
      {
        name: "nonce",
        type: "felt",
      },
    ],
    name: "write_account_nonce",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "sender",
        type: "felt",
      },
      {
        name: "authorized",
        type: "felt",
      },
    ],
    name: "set_authorized_message_sender",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "sender_address",
        type: "felt",
      },
      {
        name: "msg_hash",
        type: "Uint256",
      },
    ],
    name: "set_authorized_pre_eip155_tx",
    outputs: [],
    type: "function",
  },
  {
    inputs: [
      {
        name: "nonce",
        type: "felt",
      },
      {
        name: "origin",
        type: "felt",
      },
      {
        name: "to",
        type: "Option",
      },
      {
        name: "gas_limit",
        type: "felt",
      },
      {
        name: "gas_price",
        type: "felt",
      },
      {
        name: "value",
        type: "Uint256",
      },
      {
        name: "data_len",
        type: "felt",
      },
      {
        name: "data",
        type: "felt*",
      },
      {
        name: "access_list_len",
        type: "felt",
      },
      {
        name: "access_list",
        type: "felt*",
      },
    ],
    name: "eth_call",
    outputs: [
      {
        name: "return_data_len",
        type: "felt",
      },
      {
        name: "return_data",
        type: "felt*",
      },
      {
        name: "success",
        type: "felt",
      },
      {
        name: "gas_used",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "nonce",
        type: "felt",
      },
      {
        name: "origin",
        type: "felt",
      },
      {
        name: "to",
        type: "Option",
      },
      {
        name: "gas_limit",
        type: "felt",
      },
      {
        name: "gas_price",
        type: "felt",
      },
      {
        name: "value",
        type: "Uint256",
      },
      {
        name: "data_len",
        type: "felt",
      },
      {
        name: "data",
        type: "felt*",
      },
      {
        name: "access_list_len",
        type: "felt",
      },
      {
        name: "access_list",
        type: "felt*",
      },
    ],
    name: "eth_estimate_gas",
    outputs: [
      {
        name: "return_data_len",
        type: "felt",
      },
      {
        name: "return_data",
        type: "felt*",
      },
      {
        name: "success",
        type: "felt",
      },
      {
        name: "required_gas",
        type: "felt",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [
      {
        name: "to",
        type: "Option",
      },
      {
        name: "gas_limit",
        type: "felt",
      },
      {
        name: "gas_price",
        type: "felt",
      },
      {
        name: "value",
        type: "Uint256",
      },
      {
        name: "data_len",
        type: "felt",
      },
      {
        name: "data",
        type: "felt*",
      },
      {
        name: "access_list_len",
        type: "felt",
      },
      {
        name: "access_list",
        type: "felt*",
      },
    ],
    name: "eth_send_transaction",
    outputs: [
      {
        name: "return_data_len",
        type: "felt",
      },
      {
        name: "return_data",
        type: "felt*",
      },
      {
        name: "success",
        type: "felt",
      },
      {
        name: "gas_used",
        type: "felt",
      },
    ],
    type: "function",
  },
  {
    inputs: [
      {
        name: "from_address",
        type: "felt",
      },
      {
        name: "l1_sender",
        type: "felt",
      },
      {
        name: "to_address",
        type: "felt",
      },
      {
        name: "value",
        type: "felt",
      },
      {
        name: "data_len",
        type: "felt",
      },
      {
        name: "data",
        type: "felt*",
      },
    ],
    name: "handle_l1_message",
    outputs: [],
    type: "l1_handler",
  },
];

export const KAKAROT = new Contract(abi, KAKAROT_ADDRESS, PROVIDER);
