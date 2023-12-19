#!/bin/sh

# Compute the class hashes
PROXY_ACCOUNT_CLASS_HASH=$(starkli class-hash ${MADARA_PATH}/cairo-contracts/kakarot/proxy.json)
EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=$(starkli class-hash ${MADARA_PATH}/cairo-contracts/kakarot/externally_owned_account.json)
CONTRACT_ACCOUNT_CLASS_HASH=$(starkli class-hash ${MADARA_PATH}/cairo-contracts/kakarot/contract_account.json)

export PROXY_ACCOUNT_CLASS_HASH
export EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH
export CONTRACT_ACCOUNT_CLASS_HASH

# Start madara-bin in the background
/madara-bin \
	--rpc-external \
	--rpc-methods=unsafe \
	--rpc-cors=all \
	--tmp \
	--dev &

# Loop until the curl command succeeds
until
	curl --silent --request POST \
		--header "Content-Type: application/json" \
		--data '{
           "jsonrpc": "2.0", 
           "method": "starknet_getClassHashAt", 
           "params": [{"block_number": 0}, "0x9001"], 
           "id": 1
       }' \
		"${STARKNET_NETWORK}" # Use the provided network address
do
	echo "Waiting for Madara to start..."
	sleep 5
done

# Once Madara is ready, start RPC
/usr/local/bin/kakarot-rpc
