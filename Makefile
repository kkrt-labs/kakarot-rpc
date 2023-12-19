HURL_FILES = $(shell find ./rpc-call-examples/ -name '*.hurl')

-include .env
export

ifndef STARKNET_NETWORK
override STARKNET_NETWORK = katana
endif

setup: .gitmodules
	chmod +x ./scripts/extract_abi.sh
	git submodule update --init --recursive
	cd lib/kakarot && make setup && make build && make build-sol && cd ..
	./scripts/extract_abi.sh

deploy-kakarot:
	cd lib/kakarot && STARKNET_NETWORK=$(STARKNET_NETWORK) poetry run python ./scripts/deploy_kakarot.py && cd ..

run-dev:
	PROXY_ACCOUNT_CLASS_HASH=$(shell jq -r '.proxy' ./lib/kakarot/deployments/$(STARKNET_NETWORK)/declarations.json) CONTRACT_ACCOUNT_CLASS_HASH=$(shell jq -r '.contract_account' ./lib/kakarot/deployments/$(STARKNET_NETWORK)/declarations.json) EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=$(shell jq -r '.externally_owned_account' ./lib/kakarot/deployments/$(STARKNET_NETWORK)/declarations.json) KAKAROT_ADDRESS=$(shell jq -r '.kakarot.address' ./lib/kakarot/deployments/$(STARKNET_NETWORK)/deployments.json) RUST_LOG=trace cargo run

# Run Katana, Deploy Kakarot, Run Kakarot RPC
katana-rpc-up:
	docker compose -f docker-compose.yaml -f docker-compose.katana.yaml up -d --force-recreate --pull always

katana-rpc-down:
	docker compose -f docker-compose.yaml -f docker-compose.katana.yaml down --remove-orphans

# Run Madara, Deploy Kakarot, Run Kakarot RPC
madara-rpc-up:
	docker compose up -d --force-recreate --pull always

madara-rpc-down:
	docker compose down --remove-orphans

install-katana:
	cargo install --git https://github.com/dojoengine/dojo --locked --rev be16762 katana

run-katana: install-katana
	rm -fr .katana/ && mkdir .katana
	katana --dump-state .katana/dump.bin & echo $$! > .katana/pid

kill-katana:
	kill -2 `cat .katana/pid` && rm -fr .katana/pid

dump-katana: run-katana deploy-kakarot kill-katana

dump-genesis: build-kakarot
	cargo run --bin dump-genesis

test: dump-katana
	cargo test --all

test-coverage:
	cargo llvm-cov nextest --all-features --workspace --lcov --output-path lcov.info

test-examples:
	hurl $(HURL_FILES)

.PHONY: devnet test
