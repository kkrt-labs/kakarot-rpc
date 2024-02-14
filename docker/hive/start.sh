# 1. Convert the genesis.json from hive format to Katana format using the convert binary from Kakarot RPC test utils
# 2. Start the Katana, the CairoVM chain
echo "Launching Katana..."
katana --block-time 6000 --disable-fee --chain-id=kkrt &
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
mongod --dbpath "/usr/app/data/db" --logpath "/usr/app/data/logs/mongod.log" &
## DNA
echo "Launching DNA..."
starknet start --rpc=http://starknet:5050 --wait-for-rpc --data=/data & 
# ## Indexer
echo "Launching indexer..."
sink-mongo run /usr/src/app/code/kakarot-indexer/src/main.ts

# 4. Start the Kakarot RPC service
# echo "Launching Kakarot RPC..."
# kakarot-rpc --bin hive_genesis --features testing
#  "KAKAROT_ADDRESS=
#  "DEPLOYER_ACCOUNT_ADDRESS=
#  "PROXY_ACCOUNT_CLASS_HASH=
#  "EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=
#  "CONTRACT_ACCOUNT_CLASS_HASH=
#  Make sure they are set in the environment after Katana has created a genesis file.

# kakarot-rpc
