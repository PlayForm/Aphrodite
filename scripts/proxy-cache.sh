#!/bin/bash
# headroom cache proxy launcher — DeepSeek backend, cache-only mode
# Usage: bash scripts/proxy-cache.sh
# Port 8787 — prefix-freeze cache (no token compression)

export HEADROOM_BACKEND=anyllm
export HEADROOM_ANYLLM_PROVIDER=openai
export OPENAI_API_KEY="***REMOVED***"
export OPENAI_TARGET_API_URL="https://api.deepseek.com/v1"
export HEADROOM_PORT=8787

exec headroom proxy --port "$HEADROOM_PORT" --no-optimize