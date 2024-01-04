#!/usr/bin/env bash

IFS=',' read -ra PARTS <<< "$ENDPOINTS"
ARGUMENTS=""

for part in "${PARTS[@]}"; do
  ARGUMENTS+="--endpoint $part "
done

if [ -n "$REDIS_URL" ]; then
  ARGUMENTS+="--redis-url $REDIS_URL"
  # wait for redis to be ready
  sleep 1
fi

exec $1 --port 8124 --bind 0.0.0.0 $ARGUMENTS
