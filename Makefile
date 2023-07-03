# Read from the .env file
include .env
export

# TODO: Would want to use this to ensure the build command only runs the first time
KAKAROT_BUILD_FILES = $(wildcard $(COMPILED_KAKAROT_PATH)/*)
HURL_FILES = $(shell find ./rpc-call-examples/ -name '*.hurl')

setup: .gitmodules
	git submodule update --init --recursive
	cd kakarot && make setup

kakarot-build: setup 
	cd kakarot && make build

# install dependencies, automatically creates a virtual environment
poetry-install: 
	poetry install

# run devnet
devnet: poetry-install 
	poetry run starknet-devnet --seed 0 --disable-rpc-request-validation --load-path deployments/devnet.pkl --timeout 5000

# build
build:
	cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# run
run: 
	source .env && RUST_LOG=debug cargo run -p kakarot-rpc

#run-release
run-release:
	source .env && cargo run --release -p kakarot-rpc

test: kakarot-build
	cargo test --all

test-examples:
	hurl $(HURL_FILES)

.PHONY: install run devnet test
