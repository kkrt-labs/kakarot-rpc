#install kakarot

install:
	sh kakarotup

# install dependencies, automatically creates a virtual environment
poetry-install:
	poetry install

# run devnet
devnet: poetry-install 
	 poetry run starknet-devnet --seed 0 --disable-rpc-request-validation --load-path deployments/devnet/devnet.pkl --timeout 300

# build
build:
	cargo build --all --release

# run
run: 
	source .env && RUST_LOG=debug cargo run -p kakarot_rpc

#run-release
run-release:
	source .env && cargo run --release -p kakarot_rpc

test:
	cargo test --all

.PHONY: install run devnet test