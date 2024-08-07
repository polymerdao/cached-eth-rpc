#!/bin/sh

# Ensure the script exits if any command fails
set -e

# Check if required environment variables are set
if [ -z "$ETH_SEPOLIA_RPC" ] || [ -z "$OP_SEPOLIA_RPC" ] || [ -z "$BASE_SEPOLIA_RPC" ]; then
  echo "Error: One or more required environment variables are not set."
  echo "Please set ETH_SEPOLIA_RPC, OP_SEPOLIA_RPC, and BASE_SEPOLIA_RPC."
  exit 1
fi

# Run the cached-eth-rpc command with environment variables
exec /app/cached-eth-rpc --bind 0.0.0.0 --port 8080 \
  --endpoint=eth-sepolia="$ETH_SEPOLIA_RPC" \
  --endpoint=op-sepolia="$OP_SEPOLIA_RPC" \
  --endpoint=base-sepolia="$BASE_SEPOLIA_RPC"
