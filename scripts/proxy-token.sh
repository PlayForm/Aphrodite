#!/bin/bash
# headroom token proxy launcher — DeepSeek backend, full compression
# API key: set HEADROOM_DEEPSEEK_KEY env var or export OPENAI_API_KEY
# Usage: bash scripts/proxy-token.sh
# Port 8788 — token compression mode (SmartCrusher + Kompress active)

export HEADROOM_BACKEND=anyllm
export HEADROOM_ANYLLM_PROVIDER=openai
export OPENAI_API_KEY="${HEADROOM_DEEPSEEK_KEY:-$OPENAI_API_KEY}"
export OPENAI_TARGET_API_URL="https://api.deepseek.com/v1"
export HEADROOM_PORT=8788

exec headroom proxy --port "$HEADROOM_PORT"