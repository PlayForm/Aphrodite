#!/bin/bash
# dev-dual.sh - launch both aphrodite Rust proxy modes (quiet)
cd "$(dirname "$0")/.." || exit 1
trap 'kill 0' SIGINT SIGTERM EXIT
KEY="${APHRODITE_API_KEY:-}"
[ -z "$KEY" ] && {
	echo "Set APHRODITE_API_KEY"
	exit 1
}
echo ":9797 cache | :9798 token"
RUST_LOG=aphrodite=info,tower_http=warn cargo run -p aphrodite -- --mode cache --listen 127.0.0.1:9797 --api-key "$KEY" &
PID1=$!
RUST_LOG=aphrodite=info,tower_http=warn cargo run -p aphrodite -- --mode token --listen 127.0.0.1:9798 --api-key "$KEY" --tool-relay &
PID2=$!
wait $PID1 $PID2
