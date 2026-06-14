#!/bin/bash
# ─── Headroom Proxy Starter ───────────────────────────────────────────
# Launches headroom in proxy mode with compression enabled.
#
# The proxy compresses Anthropic-format tool outputs (tool_result blocks).
# OpenAI-format messages (role=tool) pass through uncompressed - use the
# inline Python library (Compress.compress()) for OpenAI-format compression.
#
# Usage:
#   bash scripts/proxy-launch.sh          # start proxy (cache mode)
#   bash scripts/proxy-launch.sh token    # token mode (more aggressive)
#   bash scripts/proxy-launch.sh cache 9090  # custom port
#
# Env vars needed (set in .env):
#   HEADROOM_DEEPSEEK_KEY  - DeepSeek API key
#   OPENAI_BASE_URL        - defaults to https://api.deepseek.com
#   HEADROOM_CODE_AWARE_ENABLED - set to "true" for AST code compression
# ──────────────────────────────────────────────────────────────────────

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
MODE="${1:-cache}"
PORT="${2:-8787}"
HEADROOM_BIN="$REPO_DIR/.venv/bin/headroom"
DEEPSEEK_URL="${OPENAI_BASE_URL:-https://api.deepseek.com}"

if [ -f "$REPO_DIR/.env" ]; then
    set -a; source "$REPO_DIR/.env"; set +a
fi

if [ -z "${HEADROOM_DEEPSEEK_KEY}" ]; then
    echo "ERROR: HEADROOM_DEEPSEEK_KEY not set. Add to $REPO_DIR/.env" >&2
    exit 1
fi

echo "Starting headroom proxy on :${PORT} (mode=${MODE})"
echo "  Backend: openai → ${DEEPSEEK_URL}"
echo "  Compression: enabled (code-aware=${HEADROOM_CODE_AWARE_ENABLED:-false})"

# Kill existing proxy on this port
pkill -f "headroom proxy.*${PORT}" 2>/dev/null || true
sleep 0.5

exec env \
    OPENAI_BASE_URL="$DEEPSEEK_URL" \
    OPENAI_API_KEY="${HEADROOM_DEEPSEEK_KEY}" \
    HEADROOM_CODE_AWARE_ENABLED="${HEADROOM_CODE_AWARE_ENABLED:-true}" \
    "$HEADROOM_BIN" proxy \
        --port "$PORT" \
        --mode "$MODE" \
        --backend openai \
        --openai-api-url "$DEEPSEEK_URL" \
        --code-aware
