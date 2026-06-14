#!/bin/bash
# headroom token proxy — source .env for keys, then launch
# Port 8788 — full compression (SmartCrusher + Kompress)
source ~/.hermes/.env 2>/dev/null
export HEADROOM_BACKEND=anyllm
export HEADROOM_ANYLLM_PROVIDER=openai
export OPENAI_TARGET_API_URL="https://api.deepseek.com/v1"
export HEADROOM_PORT=8788
exec headroom proxy --port "$HEADROOM_PORT"