# 1. Create the genesis file
echo "Supplied genesis state:"
cat /genesis.json
mv /genesis.json /genesis/hive-genesis.json
echo "Creating the genesis file..."
hive_genesis \
	-k genesis/contracts \
	--hive-genesis genesis/hive-genesis.json \
	-g genesis.json \
	-m manifest.json
mv /genesis/hive-genesis.json /hive-genesis.json && rm -fr /genesis

# 2. Start Katana
echo "Launching Katana..."
chain_id=$(printf '%x' $(jq -r '.config.chainId' hive-genesis.json))
katana --block-time 6000 --disable-fee --chain-id=0x$chain_id --genesis genesis.json &
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

export UNINITIALIZED_ACCOUNT_CLASS_HASH=$(jq -r '.declarations.uninitialized_account' manifest.json)
export ACCOUNT_CONTRACT_CLASS_HASH=$(jq -r '.declarations.account_contract' manifest.json)
export KAKAROT_ADDRESS=$(jq -r '.deployments.kakarot_address' manifest.json)

# Only launch the Hive Chain if the chain file exists
if test -f "/chain.rlp"; then
	echo "Launching Hive Chain..."
	# THIS needs to be changed if Katana ever updates their predeployed accounts
	hive_chain \
		--chain-path /chain.rlp \
		--relayer-address 0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca \
		--relayer-pk 0x2bbf4f9fd0bbb2e60b0316c1fe0b76cf7a4d0198bd493ced9b8df2a3a24d68a
fi

# 3. Start the Indexer service: DNA Indexer, Indexer transformer, and MongoDB
## MongoDB
echo "Launching mongo..."
mongod --bind_ip 0.0.0.0 --noauth &
## DNA
echo "Launching DNA..."
starknet start --rpc=http://localhost:5050 --wait-for-rpc --head-refresh-interval-ms=300 --data=/data &
# ## Indexer
echo "Launching indexer..."
sink-mongo run /usr/src/app/code/indexer/src/main.ts &

### 3.5. Await the Indexer to be healthy
echo "Waiting for the indexer to start..."
sleep 9

# 4. Start the Kakarot RPC service
echo "Launching Kakarot RPC..."
# THIS needs to be changed if Katana ever updates their predeployed accounts
kakarot-rpc
