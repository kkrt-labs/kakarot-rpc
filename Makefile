-include .env
export

ifndef STARKNET_NETWORK
override STARKNET_NETWORK = katana
endif

MANIFEST=.katana/manifest.json

usage:
	@echo "Usage:"
	@echo "    setup:           Setup the project by setting the Kakarot submodule, compiling solidity contracts and extracting Starknet contracts abis."
	@echo "    deploy-kakarot:  Deploys kakarot. Uses the STARKNET_NETWORK environment variable to determine the network."
	@echo "    load-env:        Loads environment variables necessary for RPC."
	@echo "    run-dev:         Run the development version of the Kakarot RPC server."
	@echo "    install-katana:  Install Katana from the dojoengine."
	@echo "    katana-genesis:  Generates a new genesis block for Katana."
	@echo "    run-katana:      Runs Katana with Kakarot deployed in the genesis."
	@echo "    test:            Runs all tests."
	@echo "    test-target:     Run a specific test target. Requires katana-genesis to have ran once before."
	@echo "    benchmark:       Executes TPS benchmarks."
	@echo "    docker-build:    Builds the Kakarot RPC docker image."
	@echo "    local-rpc-up:    Runs a local instance of the entire Kakarot stack: RPC, Indexer, Starknet client, Kakarot contracts deployed. This is equivalent to running a local anvil."
	@echo "    testnet-rpc-up:  Runs a local instance of the Kakarot RPC layer, pointing to the Kakarot Sepolia Testnet in production."
	@echo "    staging-rpc-up:  Runs a local instance of the Kakarot RPC layer, pointing to the Kakarot Staging environment."

setup: .gitmodules
	chmod +x ./scripts/extract_abi.sh
	git submodule update --init --recursive
	cp .env.example .env
	cd lib/kakarot && uv sync --all-extras --dev && make build && make build-sol && \
	mv build/ssj/contracts_Cairo1Helpers.contract_class.json build/cairo1_helpers.json && rm -fr build/ssj
	./scripts/extract_abi.sh

deploy-kakarot:
	cd lib/kakarot && STARKNET_NETWORK=$(STARKNET_NETWORK) poetry run python ./kakarot_scripts/deploy_kakarot.py && cd ..

load-env:
	$(eval UNINITIALIZED_ACCOUNT_CLASS_HASH=$(shell jq -r '.declarations.uninitialized_account' $(MANIFEST)))
	$(eval ACCOUNT_CONTRACT_CLASS_HASH=$(shell jq -r '.declarations.account_contract' $(MANIFEST)))
	$(eval KAKAROT_ADDRESS=$(shell jq -r '.deployments.kakarot_address' $(MANIFEST)))

run-dev: load-env
	RUST_LOG=trace cargo run --bin kakarot-rpc

install-katana:
	cargo install --git https://github.com/dojoengine/dojo --locked --tag v1.0.0-alpha.14 katana

katana-genesis: install-katana
	cargo run --bin katana_genesis --features testing

run-katana: katana-genesis
	katana --disable-fee --chain-id=kkrt --genesis .katana/genesis.json

test: katana-genesis load-env
	cargo test --all --features testing

test-ci: load-env
	cargo nextest run --all --features testing --profile ci

# Example: `make test-target TARGET=test_raw_transaction`
test-target: load-env
	cargo test --tests --all-features $(TARGET) -- --nocapture

benchmark:
	cd benchmarks && bun i && bun run benchmark

docker-build: setup
	docker build -t kakarot-rpc . -f docker/rpc/Dockerfile

local-rpc-up:
	docker compose up -d --force-recreate

testnet-rpc-up:
	docker compose -f docker-compose.prod.yaml up -d --force-recreate

staging-rpc-up:
	docker compose -f docker-compose.staging.yaml up -d --force-recreate

# To stop the dockerized local instances, run: docker compose -f <NAME OF THE FILE> down
# To delete volumes, add the `--volumes` flag to the command above.
# Example: docker compose -f docker-compose.prod.yaml down --remove-orphans --volumes

.PHONY: test
