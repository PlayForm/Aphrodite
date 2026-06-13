#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
MODE="${1:-cache}"
PORT="${2:-8787}"
HEADROOM_BIN="$REPO_DIR/.venv/bin/headroom"
DEEPSEEK_URL="https://api.deepseek.com"

if [ -f "$REPO_DIR/.env" ]; then source "$REPO_DIR/.env"; fi

pkill -f "headroom proxy.*${PORT}" 2>/dev/null || true
sleep 0.5

exec env \
    OPENAI_BASE_URL="$DEEPSEEK_URL" \
    OPENAI_API_KEY="${HEADROOM_DEEPSEEK_KEY}" \
    "$HEADROOM_BIN" proxy \
        --port "$PORT" \
        --mode "$MODE" \
        --backend openai \
        --openai-api-url "$DEEPSEEK_URL"
