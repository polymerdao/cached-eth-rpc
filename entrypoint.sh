#!/bin/sh

get_endpoints() {
  # Loop through environment variables that match the pattern 'CER_*_RPC'
  for var in $(env | grep -E '^CER_.*_RPC=' | sed 's/=.*//'); do
      # Remove the 'CER_' prefix and the '_RPC' suffix
      endpoint_name=$(echo "$var" | sed 's/^CER_//' | sed 's/_RPC$//')

      # Replace underscores with dashes in the endpoint name
      endpoint_name=$(echo "$endpoint_name" | tr '[:upper:]_' '[:lower:]-')

      # Append the corresponding argument to the command
      endpoints="$endpoints --endpoint=$endpoint_name=${!var}"
  done
  echo "$endpoints"
}

endpoints="$(get_endpoints)"

# Ensure the script exits if any command fails
set -e

# Run the cached-eth-rpc command with environment variables
exec /app/cached-eth-rpc "$endpoints" "$@"