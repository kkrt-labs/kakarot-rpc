#!/bin/sh

# Function to display usage
usage() {
	echo "Usage: $0 [COMMAND] [OPTION]"
	echo "Commands:"
	echo "  test         Run tests"
	echo "  deploy       Run deployments"
	echo "Options:"
	echo "  --staging    Use staging environment for command"
	echo "  --production Use production environment for command"
	exit 1
}

# Check if at least one argument is passed
if [ $# -lt 2 ]; then
	echo "Please provide at least one command and one environment"
	usage
fi

# Initialize flags
run_test=false
run_deploy=false
ENV=""

# Parse arguments
for arg in "$@"; do
	case "${arg}" in
	test)
		run_test=true
		;;
	deploy)
		run_deploy=true
		;;
	--staging)
		ENV="staging"
		;;
	--production)
		ENV="sepolia"
		;;
	*)
		echo "Unknown argument: ${arg}"
		usage
		;;
	esac
done

# Check if the environment is provided
if [ -z "${ENV}" ]; then
	echo "Please provide the environment to test against"
	exit 1
fi

cd ../lib/kakarot || exit
export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION="python"
export STARKNET_NETWORK="kakarot-${ENV}"

# Set the environment variables based on the provided environment
if [ "${ENV}" = "staging" ]; then
	export EVM_PRIVATE_KEY="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
	export KAKAROT_STAGING_RPC_URL="https://juno-kakarot-testnet-stage.karnot.xyz"
	export KAKAROT_STAGING_ACCOUNT_ADDRESS="0x7ecf6cd45c32ce84812e660cc176cb8b4de2e7a6d5916fe326bf871466fbe02"
	export WEB3_HTTP_PROVIDER_URI="https://kkrt-rpc-kakarot-testnet-stage.karnot.xyz"
	export KAKAROT_STAGING_ACCOUNT_ADDRESS="0x7ecf6cd45c32ce84812e660cc176cb8b4de2e7a6d5916fe326bf871466fbe02"
	if [ -z "${KAKAROT_STAGING_PRIVATE_KEY}" ]; then
		echo "Please provide the KAKAROT_STAGING_PRIVATE_KEY environment variable. The private key should be loaded using gpg: gpg -r recipient@kakarot.org --decrypt path/to/encrypted/key.gpg"
		exit 1
	fi

	SKIP="--ignore tests/end_to_end/L1L2Messaging"
elif [ "${ENV}" = "sepolia" ]; then
	export KAKAROT_SEPOLIA_RPC_URL="https://juno-kakarot-dev.karnot.xyz/"
	export KAKAROT_SEPOLIA_ACCOUNT_ADDRESS="0x43ABAA073C768EBF039C0C4F46DB9ACC39E9EC165690418060A652AAB39E7D8"
	export WEB3_HTTP_PROVIDER_URI="https://sepolia-rpc.kakarot.org"
	if [ -z "${EVM_PRIVATE_KEY}" ]; then
		echo "Please provide the EVM_PRIVATE_KEY environment variable"
		exit 1
	fi
	if [ -z "${KAKAROT_SEPOLIA_PRIVATE_KEY}" ]; then
		echo "Please provide the KAKAROT_SEPOLIA_PRIVATE_KEY environment variable. The private key should be loaded using gpg: gpg -r recipient@kakarot.org --decrypt path/to/encrypted/key.gpg"
		exit 1
	fi

	SKIP="--ignore tests/end_to_end/L1L2Messaging --ignore tests/end_to_end/test_kakarot.py --ignore tests/end_to_end/CairoPrecompiles -k 'not test_should_return_starknet_timestamp'"
fi

# Deploy the contracts if the deploy command is provided
if ${run_deploy}; then
	echo "Deploying the contracts to the ${ENV} environment"

	make setup && make build-sol && make build && make fetch-ssj-artifacts && make build-cairo1
	poetry run python ./kakarot_scripts/deploy_kakarot.py
fi

# Run the tests if the test command is provided
if ${run_test}; then
	echo "Running tests for the ${ENV} environment. Skipping: ${SKIP}"

	KAKAROT_ADDRESS=$(jq -r '.kakarot.address' ./deployments/kakarot-"${ENV}"/deployments.json)
	UNINITIALIZED_ACCOUNT_CLASS_HASH=$(jq -r '.uninitialized_account' ./deployments/kakarot-"${ENV}"/declarations.json)
	ACCOUNT_CONTRACT_CLASS_HASH=$(jq -r '.account_contract' ./deployments/kakarot-"${ENV}"/declarations.json)

	export KAKAROT_ADDRESS="${KAKAROT_ADDRESS}"
	export UNINITIALIZED_ACCOUNT_CLASS_HASH="${UNINITIALIZED_ACCOUNT_CLASS_HASH}"
	export ACCOUNT_CONTRACT_CLASS_HASH="${ACCOUNT_CONTRACT_CLASS_HASH}"

	eval "poetry run pytest -s tests/end_to_end ${SKIP}"
fi
