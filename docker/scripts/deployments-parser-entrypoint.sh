#!/bin/sh

echo "KAKAROT_ADDRESS=$$(jq -r '.kakarot.address' /deployments/katana/deployments.json)" > /deployments/.env;
echo "DEPLOYER_ACCOUNT_ADDRESS=$$(jq -r '.deployer_account.address' /deployments/katana/deployments.json)" >> /deployments/.env;
echo "PROXY_ACCOUNT_CLASS_HASH=$$(jq -r '.proxy' /deployments/katana/declarations.json)" >> /deployments/.env
echo "EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=$$(jq -r '.externally_owned_account' /deployments/katana/declarations.json)" >> /deployments/.env
echo "CONTRACT_ACCOUNT_CLASS_HASH=$$(jq -r '.contract_account' /deployments/katana/declarations.json)" >> /deployments/.env

exec "$@"
