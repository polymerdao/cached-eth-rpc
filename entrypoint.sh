#!/bin/sh

# Ensure the script exits if any command fails
set -e

# Check if required environment variables are set
if [ -z "$ETH_MAINNET_RPC" ] || [ -z "$OP_MAINNET_RPC" ] || [ -z "$BASE_MAINNET_RPC" ] || \
   [ -z "$ETH_SEPOLIA_RPC" ] || [ -z "$OP_SEPOLIA_RPC" ] || [ -z "$BASE_SEPOLIA_RPC" ] || \
   [ -z "$PEPTIDE_RPC" ]; then
  echo "Error: One or more required environment variables are not set."
  echo "Please set ETH_MAINNET_RPC, OP_MAINNET_RPC, BASE_MAINNET_RPC,"
  echo "ETH_SEPOLIA_RPC, OP_SEPOLIA_RPC, BASE_SEPOLIA_RPC, and PEPTIDE_RPC."
  exit 1
fi

# Run the cached-eth-rpc command with environment variables
exec /app/cached-eth-rpc \
  --endpoint=eth-mainnet="$ETH_MAINNET_RPC" \
  --endpoint=op-mainnet="$OP_MAINNET_RPC" \
  --endpoint=base-mainnet="$BASE_MAINNET_RPC" \
  --endpoint=eth-sepolia="$ETH_SEPOLIA_RPC" \
  --endpoint=op-sepolia="$OP_SEPOLIA_RPC" \
  --endpoint=base-sepolia="$BASE_SEPOLIA_RPC" \
  --endpoint=peptide="$PEPTIDE_RPC" \
 "$@"