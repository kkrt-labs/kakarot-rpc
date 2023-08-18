
./scripts/deploy_kakarot.sh
exit_status=$?

if [[ $exit_status -eq 1 ]]; then
    echo "kakarot deployment to madara failed exiting"
    exit 1
    fi

sleep 5

# get kakarot address
KAKAROT_ADDRESS=$(jq -r '.kakarot.address' ../lib/kakarot/deployments/madara/deployments.json)

echo "starting RPC with the KAKAROT_ADDRESS=$KAKAROT_ADDRESS"

run rpc with environment variables
KAKAROT_HTTP_RPC_ADDRESS=0.0.0.0:3030 \
PROXY_ACCOUNT_CLASS_HASH=0x4b9eef81a3f0a582dfed69be93196cedbff063e0fa206b34b4c2f06ac505f0c \
RUST_LOG=error \
STARKNET_NETWORK=madara \
KAKAROT_ADDRESS=$KAKAROT_ADDRESS \
../target/release/kakarot-rpc
