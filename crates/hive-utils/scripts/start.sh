#!/bin/sh

# Compute the proxy account class hash
export PROXY_ACCOUNT_CLASS_HASH=$(starkli class-hash ${MADARA_PATH}/cairo-contracts/kakarot/proxy.json)

# Start madara-bin in the background
/madara-bin \
    --rpc-external \
    --rpc-methods=unsafe \
    --rpc-cors=all \
    --tmp \
    --dev &

# Start RPC
/usr/local/bin/kakarot-rpc
