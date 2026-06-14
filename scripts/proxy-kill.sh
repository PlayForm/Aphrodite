#!/bin/bash
# Kill headroom proxy on port 8787
lsof -ti :8787 | xargs kill -9 2>/dev/null && echo "proxy killed" || echo "no proxy running"