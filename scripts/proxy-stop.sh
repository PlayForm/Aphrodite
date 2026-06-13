#!/bin/bash
# Stop all headroom proxies
pkill -f "headroom proxy" 2>/dev/null && echo "All proxies stopped" || echo "No proxies running"
