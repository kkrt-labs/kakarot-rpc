-include .env
export

ifndef STARKNET_NETWORK
override STARKNET_NETWORK = katana
endif

MANIFEST=.katana/manifest.json

# Setup the project. Will also rename the precompiles compiled class
# and move it to the correct location.
setup: .gitmodules
	chmod +x ./scripts/extract_abi.sh
	git submodule update --init --recursive
	cd lib/kakarot && make setup && make build && make build-sol && \
	mv build/ssj/contracts_Cairo1Helpers.contract_class.json build/cairo1_helpers.json && rm -fr build/ssj && cd ..
	./scripts/extract_abi.sh

deploy-kakarot:
	cd lib/kakarot && STARKNET_NETWORK=$(STARKNET_NETWORK) poetry run python ./kakarot_scripts/deploy_kakarot.py && cd ..

load-env:
	$(eval UNINITIALIZED_ACCOUNT_CLASS_HASH=$(shell jq -r '.declarations.uninitialized_account' $(MANIFEST)))
	$(eval ACCOUNT_CONTRACT_CLASS_HASH=$(shell jq -r '.declarations.account_contract' $(MANIFEST)))
	$(eval KAKAROT_ADDRESS=$(shell jq -r '.deployments.kakarot_address' $(MANIFEST)))

run-dev: load-env
	RUST_LOG=trace cargo run --bin kakarot-rpc


### Kakarot tests and local development:
install-katana:
	cargo install --git https://github.com/dojoengine/dojo --locked --tag v0.6.1-alpha.4 katana

katana-genesis: install-katana
	rm -fr .katana/ && mkdir .katana
	cargo run --bin katana_genesis --features testing

# Runs Katana with Kakarot deployed on top.
run-katana: katana-genesis
	katana --disable-fee --chain-id=kkrt --genesis .katana/genesis.json

# Total test suite.
test: katana-genesis load-env
	cargo test --all --features testing

# Run a specific test target. Need to run make katana-genesis once first.
# Example: `make test-target TARGET=test_raw_transaction`
test-target: load-env
	cargo test --tests --features "testing,hive" $(TARGET) -- --nocapture

benchmark:
	cd benchmarks && bun i && bun run benchmark


### Running the Kakarot stack locally:

docker-build: setup
	docker build -t kakarot-rpc . -f docker/rpc/Dockerfile

# Runs a local instance of the entire Kakarot stack: RPC, Indexer, Starknet client, Kakarot contracts deployed.
# This is equivalent to running a local anvil.
local-rpc-up:
	docker compose up -d --force-recreate

# Runs a local instance of the Kakarot RPC layer, pointing to the Kakarot Sepolia Testnet in production
# This is equivalent to locally running a Geth instance that syncs with Sepolia.
testnet-rpc-up:
	docker compose -f docker-compose.prod.yaml up -d --force-recreate

# Runs a local instance of the Kakarot RPC layer, pointing to the Kakarot Staging environment.
# The staging environment is in all things equivalent to the production environment, but with a different chain ID.
# It is meant to be used for testing and development before pushing things to the public production-ready Testnet.
staging-rpc-up:
	docker compose -f docker-compose.staging.yaml up -d --force-recreate

# To stop the dockerized local instances, run: docker compose -f <NAME OF THE FILE> down
# To delete volumes, add the `--volumes` flag to the command above.
# Example: docker compose -f docker-compose.prod.yaml down --remove-orphans --volumes


.PHONY: test
