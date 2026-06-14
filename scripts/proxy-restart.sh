#!/bin/bash
# Restart headroom proxy with optional overrides
# Usage: bash scripts/proxy-restart.sh [--token] [--port 8788] [--backend anthropic]

MODE="cache"
PORT=8787

while [[ $# -gt 0 ]]; do
    case "$1" in
        --token) MODE="token"; shift ;;
        --port) PORT="$2"; shift 2 ;;
        --backend) BACKEND_OVERRIDE="$2"; shift 2 ;;
        *) echo "unknown: $1"; exit 1 ;;
    esac
done

# Kill existing
lsof -ti ":$PORT" | xargs kill -9 2>/dev/null
sleep 1

export HEADROOM_BACKEND="${BACKEND_OVERRIDE:-anyllm}"
export HEADROOM_ANYLLM_PROVIDER=openai
export OPENAI_API_KEY="sk-d0fcac00ae75413790790864ce39893c"
export OPENAI_TARGET_API_URL="https://api.deepseek.com/v1"
export HEADROOM_PORT="$PORT"

FLAGS="--port $PORT"
[[ "$MODE" == "cache" ]] && FLAGS="$FLAGS --no-optimize"

exec headroom proxy $FLAGS