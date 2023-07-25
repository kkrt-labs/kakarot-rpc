#!/bin/sh

# Parse the JSON files and export the results to an environment file
KAKAROT_ADDRESS=$(jq -r '.kakarot.address' /app/deployments/deployments.json)
PROXY_ACCOUNT_CLASS_HASH=$(jq -r '.proxy' /app/deployments/declarations.json)

echo "export KAKAROT_ADDRESS=$KAKAROT_ADDRESS" > /app/deployments/.env
echo "export PROXY_ACCOUNT_CLASS_HASH=$PROXY_ACCOUNT_CLASS_HASH" >> /app/deployments/.env
