HURL_FILES = $(shell find ./rpc-call-examples/ -name '*.hurl')

STARKNET_NETWORK?=madara

-include .env
export

pull-kakarot: .gitmodules 
	git submodule update --init --recursive
	cd lib/kakarot && make setup

build-kakarot: setup 
	cd lib/kakarot && make build && make build-sol

build-and-deploy-kakarot:
	cd lib/kakarot && STARKNET_NETWORK=$(STARKNET_NETWORK) make deploy

deploy-kakarot:
	cd lib/kakarot && STARKNET_NETWORK=$(STARKNET_NETWORK) poetry run python ./scripts/deploy_kakarot.py

setup: pull-kakarot build-kakarot

# run devnet
devnet: 
	docker run --rm -it -p 5050:5050 -v $(PWD)/deployments:/app/kakarot/deployments -e STARKNET_NETWORK=katana ghcr.io/kkrt-labs/kakarot/katana:latest

# build
build:
	cargo build --all --release

# run
run: 
	cargo run -p kakarot-rpc

run-dev:
	KAKAROT_ADDRESS=$(shell jq -r '.kakarot.address' ./lib/kakarot/deployments/$(STARKNET_NETWORK)/deployments.json) RUST_LOG=trace cargo run -p kakarot-rpc

#run-release
run-release:
	cargo run --release -p kakarot-rpc

# Run Katana, Deploy Kakarot, Run Kakarot RPC
katana-rpc-up:
	docker-compose -f docker-compose.yaml -f docker-compose.katana.yaml up -d --force-recreate

katana-rpc-down:
	docker-compose -f docker-compose.yaml -f docker-compose.katana.yaml down --remove-orphans

# Run Madara, Deploy Kakarot, Run Kakarot RPC
madara-rpc-up:
	docker-compose up -d --force-recreate

madara-rpc-down:
	docker-compose down --remove-orphans

test:
	cargo test --all

test-coverage:
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

test-examples:
	hurl $(HURL_FILES)

.PHONY: install run devnet test
