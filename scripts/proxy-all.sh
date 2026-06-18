#!/usr/bin/env bash
# proxy-all.sh - launch all headroom proxies
# :8787 (Python cache), :8788 (Rust token), :9797 (Rust cache), :9798 (Rust token)
#
# Usage:
#   ./scripts/proxy-all.sh              # start all
#   ./scripts/proxy-all.sh --stop       # stop all
#   ./scripts/proxy-all.sh --status     # check all

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1:-}" in
--stop)
	bash "$SCRIPT_DIR/proxy-stop.sh"
	bash "$SCRIPT_DIR/proxy-9797.sh" --stop 2>/dev/null || true
	bash "$SCRIPT_DIR/proxy-9798.sh" --stop 2>/dev/null || true
	echo "All proxies stopped"
	exit 0
	;;
--status)
	echo "=== :8787 (Python cache) ==="
	python3 "$SCRIPT_DIR/proxy-start.py" --port 8787 --stop 2>/dev/null
	curl -sf http://127.0.0.1:8787/health && echo "UP" || echo "DOWN"
	echo ""
	echo "=== :8788 (Rust token) ==="
	bash "$SCRIPT_DIR/proxy-token.sh" --status 2>/dev/null || echo "DOWN"
	echo ""
	echo "=== :9797 (Rust cache) ==="
	bash "$SCRIPT_DIR/proxy-9797.sh" --status 2>/dev/null || echo "DOWN"
	echo ""
	echo "=== :9798 (Rust token) ==="
	bash "$SCRIPT_DIR/proxy-9798.sh" --status 2>/dev/null || echo "DOWN"
	exit 0
	;;
esac

echo "Launching all headroom proxies..."
echo ""

# :8787 - Python cache
echo "► :8787 (Python cache, code-aware)"
python3 "$SCRIPT_DIR/proxy-start.py" --port 8787 --mode cache &
sleep 2

# :8788 - Rust token
echo "► :8788 (Rust token, CCR)"
bash "$SCRIPT_DIR/proxy-token.sh" &
sleep 2

# :9797 - Rust cache (new)
echo "► :9797 (Rust cache, in-memory CCR)"
bash "$SCRIPT_DIR/proxy-9797.sh" &
sleep 2

# :9798 - Rust token (new, tool relay)
echo "► :9798 (Rust token, tool relay)"
bash "$SCRIPT_DIR/proxy-9798.sh" &
sleep 2

echo ""
echo "All proxies launched. Check status with: $0 --status"
