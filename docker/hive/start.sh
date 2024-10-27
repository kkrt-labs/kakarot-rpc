#!/bin/bash

# Function to handle errors
function handle_error() {
    echo "Error occurred in ${FUNCNAME[1]} at line ${BASH_LINENO[0]}."
    exit 1
}

# 1. Create the genesis file
function create_genesis() {
    echo "Supplied genesis state:"
    cat /genesis.json || handle_error
    mv /genesis.json /genesis/hive-genesis.json || handle_error
    echo "Creating the genesis file..."
    hive_genesis \
        -k genesis/contracts \
        --hive-genesis genesis/hive-genesis.json \
        -g genesis.json \
        -m manifest.json || handle_error
    mv /genesis/hive-genesis.json /hive-genesis.json && rm -fr /genesis || handle_error
}

# 2. Start Katana
function start_katana() {
    echo "Launching Katana..."
    local chain_id
    chain_id=$(printf '%x' "$(jq -r '.config.chainId' hive-genesis.json)") || handle_error
    katana --block-time 6000 --disable-fee --chain-id=0x$chain_id --genesis genesis.json &

    # 2.5. Await Katana to be healthy
    echo "Waiting for Katana to start..."
    local retries=30
    while ((retries > 0)); do
        if curl --silent --request POST \
            --header "Content-Type: application/json" \
            --data '{
               "jsonrpc": "2.0",
               "method": "starknet_blockNumber",
               "params": [],
               "id": 1
           }' \
            "${STARKNET_NETWORK}"; then
            echo "Katana is up and running."
            break
        fi
        echo "Katana not yet ready. Retrying in 1 second..."
        sleep 1
        ((retries--))
    done

    if ((retries == 0)); then
        echo "Katana did not start in the expected time."
        exit 1
    fi
}

# Exported variables from manifest.json
function set_manifest_variables() {
    export UNINITIALIZED_ACCOUNT_CLASS_HASH=$(jq -r '.declarations.uninitialized_account' manifest.json)
    export ACCOUNT_CONTRACT_CLASS_HASH=$(jq -r '.declarations.account_contract' manifest.json)
    export KAKAROT_ADDRESS=$(jq -r '.deployments.kakarot_address' manifest.json)
}

# 3. Launch Hive Chain if chain file exists
function launch_hive_chain() {
    if test -f "/chain.rlp"; then
        echo "Launching Hive Chain..."
        hive_chain \
            --chain-path /chain.rlp \
            --relayer-address 0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca \
            --relayer-pk 0x2bbf4f9fd0bbb2e60b0316c1fe0b76cf7a4d0198bd493ced9b8df2a3a24d68a || handle_error
    fi
}

# 4. Start the Indexer and MongoDB
function start_services() {
    echo "Launching MongoDB..."
    mongod --bind_ip 0.0.0.0 --noauth &

    echo "Launching DNA..."
    starknet start --rpc=http://localhost:5050 --wait-for-rpc --head-refresh-interval-ms=300 --data=/data &

    echo "Launching Indexer..."
    sink-mongo run /usr/src/app/code/indexer/src/main.ts &
    
    echo "Waiting for the Indexer to start..."
    sleep 9
}

# 5. Start Kakarot RPC service
function start_kakarot_rpc() {
    echo "Launching Kakarot RPC..."
    kakarot-rpc || handle_error
}

# Main Script Execution
create_genesis
start_katana
set_manifest_variables
launch_hive_chain
start_services
start_kakarot_rpc

echo "Script execution completed."
