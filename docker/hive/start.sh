# 1. Create the genesis file
echo "Creating the genesis file..."
KAKAROT_CONTRACTS_PATH="genesis/contracts" \
HIVE_GENESIS_PATH="genesis/hive-genesis.json" \
GENESIS_OUTPUT="genesis.json" \
MANIFEST_OUTPUT="manifest.json" \
hive_genesis;
mv /genesis/hive-genesis.json /hive-genesis.json && rm -fr /genesis

# 2. Start Katana
echo "Launching Katana..."
katana --block-time 6000 --disable-fee --chain-id=0x$(jq -r '.config.chainId' hive-genesis.json) --genesis genesis.json &
###### 2.5. Await Katana to be healthy
# Loop until the curl command succeeds
until
	curl --silent --request POST \
		--header "Content-Type: application/json" \
		--data '{
           "jsonrpc": "2.0", 
           "method": "starknet_blockNumber", 
           "params": [], 
           "id": 1
       }' \
		"${STARKNET_NETWORK}" # Use the provided network address
do
	echo "Waiting for Katana to start..."
	sleep 1
done

# 3. Start the Indexer service: DNA Indexer, Indexer transformer, and MongoDB
## MongoDB
echo "Launching mongo..."
mongod --bind_ip 0.0.0.0 --noauth &
## DNA
echo "Launching DNA..."
starknet start --rpc=http://localhost:5050 --wait-for-rpc --data=/data & 
# ## Indexer
echo "Launching indexer..."
sink-mongo run /usr/src/app/code/kakarot-indexer/src/main.ts &

# 4. Start the Kakarot RPC service
echo "Launching Kakarot RPC..."
export PROXY_ACCOUNT_CLASS_HASH=$(jq -r '.declarations.proxy' manifest.json)
export CONTRACT_ACCOUNT_CLASS_HASH=$(jq -r '.declarations.contract_account' manifest.json)
export EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=$(jq -r '.declarations.externally_owned_account' manifest.json)
export KAKAROT_ADDRESS=$(jq -r '.deployments.kakarot_address' manifest.json)
kakarot-rpc
