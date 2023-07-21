#!/bin/bash

# This script is used to set the environment variables for the Kakarot RPC
export KAKAROT_ADDRESS=$(jq -r '.kakarot.address' ./deployments/deployments.json)
export PROXY_ACCOUNT_CLASS_HASH=$(jq -r '.proxy' ./deployments/declarations.json)

echo "Starknet Network: $STARKNET_NETWORK"
echo "Kakarot address: $KAKAROT_ADDRESS"
echo "Proxy account class hash: $PROXY_ACCOUNT_CLASS_HASH"

# Run the command passed to the docker run
exec "$@"
