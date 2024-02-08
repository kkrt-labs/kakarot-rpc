# 1. Convert the genesis.json from hive format to Katana format using the convert binary from Kakarot RPC test utils
# 2. Start the Katana, the CairoVM chain
katana --block-time 6000 --disable-fee --chain-id=kkrt
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
## DNA
start --rpc=http://starknet:5050 --wait-for-rpc --data=/data
## Indexer
run /usr/src/app/code/kakarot-indexer/src/main.ts
## MongoDB
mongod 
# 4. Start the Kakarot RPC service
kakarot-rpc
