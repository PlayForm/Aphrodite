#!/usr/bin/env bash
# aphrodite cache mode — launch on :9797
#
# Usage:
#   ./scripts/proxy-9797.sh              # start
#   ./scripts/proxy-9797.sh --stop       # stop
#
# Reads HEADROOM_DEEPSEEK_KEY from ~/.hermes/.env

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_DIR/crates/aphrodite/target/release/aphrodite"
# Fall back to debug build
if [ ! -x "$BINARY" ]; then
    BINARY="$PROJECT_DIR/crates/aphrodite/target/debug/aphrodite"
fi
PID_FILE="/tmp/aphrodite-9797.pid"
LOG_FILE="/tmp/aphrodite-9797.log"
PORT=9797

# Source env
if [ -f "$HOME/.hermes/.env" ]; then
    set -a
    source "$HOME/.hermes/.env"
    set +a
fi

DB="${HEADROOM_PROXY_DB_9797:-$PROJECT_DIR/.headroom/proxy-cache-ccr.db}"
TTL="${HEADROOM_PROXY_CCR_TTL:-3600}"

case "${1:-}" in
    --stop)
        if [ -f "$PID_FILE" ]; then
            PID=$(cat "$PID_FILE")
            if kill "$PID" 2>/dev/null; then
                echo "✓ aphrodite cache (:9797) stopped (pid=$PID)"
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
                echo "✓ aphrodite cache running (pid=$PID, port=$PORT)"
                exit 0
            fi
        fi
        echo "✗ aphrodite cache not running"
        exit 1
        ;;
esac

# Build if binary doesn't exist
if [ ! -x "$BINARY" ]; then
    echo "Building aphrodite (cache)..."
    source "$HOME/.cargo/env" 2>/dev/null || true
    cargo build --manifest-path "$PROJECT_DIR/crates/aphrodite/Cargo.toml"
    if [ -x "$PROJECT_DIR/crates/aphrodite/target/debug/aphrodite" ]; then
        BINARY="$PROJECT_DIR/crates/aphrodite/target/debug/aphrodite"
    fi
fi

echo "Starting aphrodite cache on :$PORT ..."
echo "  Mode:  cache"
echo "  DB:    $DB"
echo "  Log:   $LOG_FILE"

nohup "$BINARY" \
    --mode cache \
    --listen "127.0.0.1:$PORT" \
    --ccr-db-path "$DB" \
    --ccr-ttl-seconds "$TTL" \
    > "$LOG_FILE" 2>&1 &

PID=$!
echo "$PID" > "$PID_FILE"
sleep 1

if kill -0 "$PID" 2>/dev/null; then
    echo "✓ aphrodite cache started (pid=$PID, port=$PORT)"
    echo "  Health: curl http://127.0.0.1:$PORT/health"
    echo "  Stats:  curl http://127.0.0.1:$PORT/stats"
    echo "  Stop:   $0 --stop"
else
    echo "✗ Failed to start. Check $LOG_FILE"
    cat "$LOG_FILE"
    exit 1
fi
