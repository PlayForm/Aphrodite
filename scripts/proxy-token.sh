#!/usr/bin/env bash
# aphrodite: launch the Rust token-mode proxy
#
# Usage:
#   ./scripts/proxy-token.sh              # start on :8788
#   ./scripts/proxy-token.sh --stop       # stop the proxy
#
# Reads HEADROOM_DEEPSEEK_KEY from ~/.hermes/.env

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_DIR/crates/headroom-token/target/debug/aphrodite"
PID_FILE="/tmp/aphrodite.pid"
LOG_FILE="/tmp/aphrodite.log"

# Source env
if [ -f "$HOME/.hermes/.env" ]; then
    set -a
    source "$HOME/.hermes/.env"
    set +a
fi

PORT="${HEADROOM_TOKEN_PORT:-8788}"
DB="${HEADROOM_TOKEN_DB:-$PROJECT_DIR/.headroom/token-ccr.db}"
TTL="${HEADROOM_TOKEN_CCR_TTL:-3600}"

case "${1:-}" in
    --stop)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if kill "$PID" 2>/dev/null; then
                echo "✓ aphrodite stopped (pid=$PID)"
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
            if kill -0 "$PID" 2>/dev/null; then
                echo "✓ aphrodite running (pid=$PID, port=$PORT)"
                exit 0
            fi
        fi
        echo "✗ aphrodite not running"
        exit 1
        ;;
esac

# Build if binary doesn't exist
if [ ! -x "$BINARY" ]; then
    echo "Building aphrodite..."
    source "$HOME/.cargo/env" 2>/dev/null || true
    cargo build --manifest-path "$PROJECT_DIR/crates/headroom-token/Cargo.toml"
fi

echo "Starting aphrodite on :$PORT ..."
echo "  DB:   $DB"
echo "  TTL:  ${TTL}s"
echo "  Log:  $LOG_FILE"

nohup "$BINARY" \
    --listen "127.0.0.1:$PORT" \
    --ccr-db-path "$DB" \
    --ccr-ttl-seconds "$TTL" \
    > "$LOG_FILE" 2>&1 &

PID=$!
echo "$PID" > "$PID_FILE"
sleep 1

if kill -0 "$PID" 2>/dev/null; then
    echo "✓ aphrodite started (pid=$PID, port=$PORT)"
    echo "  Stats:  curl http://127.0.0.1:$PORT/stats"
    echo "  Stop:   $0 --stop"
else
    echo "✗ Failed to start. Check $LOG_FILE"
    cat "$LOG_FILE"
    exit 1
fi
