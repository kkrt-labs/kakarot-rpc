#!/bin/sh

# Function to display usage
usage() {
	echo "Usage: $0 [COMMAND] [OPTION]"
	echo "Commands:"
	echo "  test         Run tests"
	echo "  deploy       Run deployments"
	echo "Options:"
	echo "  --sepolia    Use sepolia environment for command"
	echo "  --staging    Use staging environment for command"
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
		ENV="kakarot-staging"
		;;
	--sepolia)
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
export STARKNET_NETWORK="${ENV}"

# Set the environment variables based on the provided environment
if [ "${ENV}" = "sepolia" ]; then
	# Note: you might need to modify the following values in `lib/kakarot/kakarot_scripts/constants.py`:
	#   - NETWORKS["sepolia"].rpc_url = https://juno-kakarot-sepolia.karnot.xyz
	#   - NETWORKS["sepolia"].max_wait = 40
	export STARKNET_SEPOLIA_RPC_URL="https://juno-kakarot-sepolia.karnot.xyz"
	export WEB3_HTTP_PROVIDER_URI="https://rpc-kakarot-sepolia.karnot.xyz/"
	if [ -z "${STARKNET_SEPOLIA_ACCOUNT_ADDRESS}" ]; then
		echo "Please provide the STARKNET_SEPOLIA_ACCOUNT_ADDRESS environment variable"
		exit 1
	fi
	if [ -z "${STARKNET_SEPOLIA_PRIVATE_KEY}" ]; then
		echo "Please provide the STARKNET_SEPOLIA_PRIVATE_KEY environment variable"
		exit 1
	fi
	if [ -z "${EVM_PRIVATE_KEY}" ]; then
		echo "Please provide the EVM_PRIVATE_KEY environment variable"
		exit 1
	fi
	SKIP="--ignore tests/end_to_end/L1L2Messaging --ignore tests/end_to_end/CairoPrecompiles --ignore tests/end_to_end/EvmPrecompiles --ignore tests/end_to_end/test_kakarot.py"
elif [ "${ENV}" = "kakarot-staging" ]; then
	export EVM_PRIVATE_KEY="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
	export KAKAROT_STAGING_RPC_URL="https://juno-kakarot-sepolia.karnot.xyz/"
	export KAKAROT_STAGING_ACCOUNT_ADDRESS="0x48fc2888aad166304e63af35a48e00e32da9831e49fe30a22148fdecdb7e66f"
	export WEB3_HTTP_PROVIDER_URI="https://rpc-kakarot-sepolia-staging.karnot.xyz/"
	if [ -z "${KAKAROT_STAGING_PRIVATE_KEY}" ]; then
		echo "Please provide the KAKAROT_STAGING_PRIVATE_KEY environment variable. The private key should be loaded using gpg: gpg -r recipient@kakarot.org --decrypt path/to/encrypted/key.gpg"
		exit 1
	fi

	SKIP="--ignore tests/end_to_end/L1L2Messaging"
fi

# Deploy the contracts if the deploy command is provided
if ${run_deploy}; then
	echo "Deploying the contracts to the ${ENV} environment"

	uv sync --all-extras --dev && make build-sol && make build && make fetch-ssj-artifacts && make build-cairo1
	uv run python ./kakarot_scripts/deploy_kakarot.py
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

	eval "uv run pytest -s tests/end_to_end ${SKIP}"
fi
