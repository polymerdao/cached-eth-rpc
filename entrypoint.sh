#!/bin/sh

# Ensure the script exits if any command fails
set -e

# Check if required environment variables are set
if [ -z "$ETH_SEPOLIA" ] || [ -z "$OP_SEPOLIA" ] || [ -z "$BASE_SEPOLIA" ]; then
  echo "Error: One or more required environment variables are not set."
  echo "Please set ETH_SEPOLIA, OP_SEPOLIA, and BASE_SEPOLIA."
  exit 1
fi

# Run the cached-eth-rpc command with environment variables
exec /app/cached-eth-rpc --bind 0.0.0.0 --port 8080 \
  --endpoint=eth-sepolia="$ETH_SEPOLIA" \
  --endpoint=op-sepolia="$OP_SEPOLIA" \
  --endpoint=base-sepolia="$BASE_SEPOLIA"
