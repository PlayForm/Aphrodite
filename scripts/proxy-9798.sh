#!/usr/bin/env bash
# aphrodite aphrodite mode — launch on :9798
#
# Usage:
#   ./scripts/proxy-9798.sh              # start
#   ./scripts/proxy-9798.sh --stop       # stop
#
# Reads APHRODITE_API_KEY from ~/.hermes/.env

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_DIR/crates/aphrodite/target/release/aphrodite"
if [ ! -x "$BINARY" ]; then
	BINARY="$PROJECT_DIR/crates/aphrodite/target/debug/aphrodite"
fi
PID_FILE="/tmp/aphrodite-9798.pid"
LOG_FILE="/tmp/aphrodite-9798.log"
PORT=9798

# Source env
if [ -f "$HOME/.hermes/.env" ]; then
	set -a
	source "$HOME/.hermes/.env"
	set +a
fi

DB="${HEADROOM_PROXY_DB_9798:-$PROJECT_DIR/.headroom/proxy-token-ccr.db}"
TTL="${HEADROOM_PROXY_CCR_TTL:-3600}"
NOTIFY_URL="${HEADROOM_NOTIFY_URL:-}"
NOTIFY_KEY="${HEADROOM_NOTIFY_KEY:-}"

case "${1:-}" in
	--stop)
		if [ -f "$PID_FILE" ]; then
			PID=$(cat "$PID_FILE")
			if kill "$PID" 2> /dev/null; then
				echo "✓ aphrodite token (:9798) stopped (pid=$PID)"
			fi
			rm -f "$PID_FILE"
		else
			echo "No pid file at $PID_FILE"
		fi
		exit 0
		;;
	--status)
		if [ -f "$PID_FILE" ]; then
			PID=$(cat "$PID_FILE")
			if kill -0 "$PID" 2> /dev/null; then
				echo "✓ aphrodite token running (pid=$PID, port=$PORT)"
				exit 0
			fi
		fi
		echo "✗ aphrodite token not running"
		exit 1
		;;
esac

# Build if binary doesn't exist
if [ ! -x "$BINARY" ]; then
	echo "Building aphrodite (token)..."
	source "$HOME/.cargo/env" 2> /dev/null || true
	cargo build --manifest-path "$PROJECT_DIR/crates/aphrodite/Cargo.toml"
	if [ -x "$PROJECT_DIR/crates/aphrodite/target/debug/aphrodite" ]; then
		BINARY="$PROJECT_DIR/crates/aphrodite/target/debug/aphrodite"
	fi
fi

echo "Starting aphrodite token on :$PORT ..."
echo "  Mode:         token"
echo "  Tool relay:   enabled"
echo "  DB:           $DB"
echo "  TTL:          ${TTL}s"
echo "  Notify URL:   ${NOTIFY_URL:-none}"
echo "  Log:          $LOG_FILE"

CMD=("$BINARY"
	--mode token
	--listen "127.0.0.1:$PORT"
	--ccr-db-path "$DB"
	--ccr-ttl-seconds "$TTL"
	--tool-relay
)

if [ -n "$NOTIFY_URL" ]; then
	CMD+=(--notify-url "$NOTIFY_URL")
fi
if [ -n "$NOTIFY_KEY" ]; then
	CMD+=(--notify-key "$NOTIFY_KEY")
fi

nohup "${CMD[@]}" > "$LOG_FILE" 2>&1 &

PID=$!
echo "$PID" > "$PID_FILE"
sleep 1

if kill -0 "$PID" 2> /dev/null; then
	echo "✓ aphrodite token started (pid=$PID, port=$PORT)"
	echo "  Health:    curl http://127.0.0.1:$PORT/health"
	echo "  Stats:     curl http://127.0.0.1:$PORT/stats"
	echo "  Retrieve:  curl -X POST http://127.0.0.1:$PORT/retrieve -d '{"hash":"..."}'"
	echo "  CCR list:  curl http://127.0.0.1:$PORT/ccr/list"
	echo "  Stop:      $0 --stop"
else
	echo "✗ Failed to start. Check $LOG_FILE"
	cat "$LOG_FILE"
	exit 1
fi
