-include .env
export

ifndef STARKNET_NETWORK
override STARKNET_NETWORK = katana
endif

DECLARATIONS=./lib/kakarot/deployments/$(STARKNET_NETWORK)/declarations.json
DEPLOYMENTS=./lib/kakarot/deployments/$(STARKNET_NETWORK)/deployments.json

setup: .gitmodules
	chmod +x ./scripts/extract_abi.sh
	git submodule update --init --recursive
	cd lib/kakarot && make setup && make build && make build-sol && cd ..
	./scripts/extract_abi.sh

deploy-kakarot:
	cd lib/kakarot && STARKNET_NETWORK=$(STARKNET_NETWORK) poetry run python ./scripts/deploy_kakarot.py && cd ..

load-env:
	$(eval PROXY_ACCOUNT_CLASS_HASH=$(shell jq -r '.proxy' $(DECLARATIONS)))
	$(eval CONTRACT_ACCOUNT_CLASS_HASH=$(shell jq -r '.contract_account' $(DECLARATIONS)))
	$(eval EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=$(shell jq -r '.externally_owned_account' $(DECLARATIONS)))
	$(eval KAKAROT_ADDRESS=$(shell jq -r '.kakarot.address' $(DEPLOYMENTS)))

run-dev: load-env
	RUST_LOG=trace cargo run --bin kakarot-rpc

# Run Katana, Deploy Kakarot, Run Kakarot RPC
katana-rpc-up:
	docker compose up -d --force-recreate

# Run Madara, Deploy Kakarot, Run Kakarot RPC
madara-rpc-up:
	docker compose -f docker-compose.madara.yaml up -d --force-recreate

docker-down:
	docker compose down -v --remove-orphans && docker compose rm

install-katana:
	cargo install --git https://github.com/dojoengine/dojo --locked --rev be16762 katana

run-katana: install-katana
	rm -fr .katana/ && mkdir .katana
	katana --disable-fee --chain-id=KKRT --dump-state .katana/dump.bin & echo $$! > .katana/pid

kill-katana:
	kill -2 `cat .katana/pid` && rm -fr .katana/pid

dump-katana: run-katana deploy-kakarot kill-katana

test: dump-katana load-env
	cargo test --all --features testing

test-coverage: load-env
	cargo llvm-cov nextest --all-features --workspace --lcov --output-path lcov.info

# Make sure to have a Kakarot RPC running and the correct port set in your .env and an underlying Starknet client running.
benchmark-madara:
	cd benchmarks && bun i && bun run benchmark-madara

test-target: load-env
	cargo test --tests --features testing $(TARGET) -- --nocapture

benchmark-katana:
	cd benchmarks && bun i && bun run benchmark-katana


.PHONY: test