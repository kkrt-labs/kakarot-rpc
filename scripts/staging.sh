#!/bin/sh
cd ../lib/kakarot || exit
poetry run python ./kakarot_scripts/deploy_kakarot.py

KAKAROT_ADDRESS=$(jq -r '.kakarot.address' ./deployments/kakarot-staging/deployments.json)
UNINITIALIZED_ACCOUNT_CLASS_HASH=$(jq -r '.uninitialized_account' ./deployments/kakarot-staging/declarations.json)
ACCOUNT_CONTRACT_CLASS_HASH=$(jq -r '.account_contract' ./deployments/kakarot-staging/declarations.json)

export KAKAROT_ADDRESS="${KAKAROT_ADDRESS}"
export UNINITIALIZED_ACCOUNT_CLASS_HASH="${UNINITIALIZED_ACCOUNT_CLASS_HASH}"
export ACCOUNT_CONTRACT_CLASS_HASH="${ACCOUNT_CONTRACT_CLASS_HASH}"

poetry run pytest -s tests/end_to_end --ignore tests/end_to_end/L1L2Messaging
