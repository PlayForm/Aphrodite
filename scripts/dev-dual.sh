#!/bin/bash
# dev-dual.sh — launch both aphrodite modes from one cargo watch
cd REDACTED/Developer/Application/PlayForm/HermesCompress
KEY="sk-6e9145a0199248398205a18594cc6b8d"

echo "=== APHRODITE DUAL ==="
echo ":9797 cache | :9798 token"
echo ""

# Launch both
RUST_LOG=aphrodite=debug cargo run -p aphrodite -- --mode cache --listen 127.0.0.1:9797 --api-key "$KEY" --dev &
PID_CACHE=$!

RUST_LOG=aphrodite=debug cargo run -p aphrodite -- --mode token --listen 127.0.0.1:9798 --api-key "$KEY" --tool-relay --dev &
PID_TOKEN=$!

echo "Cache PID: $PID_CACHE"
echo "Token PID: $PID_TOKEN"

# Wait for either to exit
wait -n
