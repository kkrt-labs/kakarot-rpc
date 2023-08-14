#!/bin/bash

#!/bin/bash

# Function to check RPC status with retries
check_rpc_status() {
  local RPC_ENDPOINT="http://127.0.0.1:9944/health"  
  local MAX_RETRIES=50
  local WAIT_SECONDS=5

  local retry_count=0
  while [[ $retry_count -lt $MAX_RETRIES ]]; do
    if curl --silent --fail --max-time 5 "$RPC_ENDPOINT" > /dev/null; then
      echo "Madara RPC is up and running."
      return 0  # Success
    else
      echo "Attempt $((retry_count + 1)) failed. Waiting $WAIT_SECONDS seconds before retrying..."
      sleep $WAIT_SECONDS
      ((retry_count++))
    fi
  done

  echo "Max retries reached. Unable to connect to the RPC endpoint."
  return 1  # Failure
}

check_rpc_status
exit_status=$?

if [[ $exit_status -eq 1 ]]; then
    echo "Madara RPC endpoint is not up. Exiting..."
    exit 1
    fi

cd ..

# make pull-kakarot
cp lib/kakarot/.env.example lib/kakarot/.env
cd lib/kakarot 
STARKNET_NETWORK=madara make deploy
