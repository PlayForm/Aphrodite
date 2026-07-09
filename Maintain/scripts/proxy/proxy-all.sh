#!/usr/bin/env bash
# proxy-all.sh - launch all Aphrodite Rust proxies
# :9797 (cache mode), :9798 (token mode with tool relay)
#
# Usage:
#   ./scripts/proxy-all.sh              # start all
#   ./scripts/proxy-all.sh --stop       # stop all
#   ./scripts/proxy-all.sh --status     # check all

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1:-}" in
--stop)
	bash "$SCRIPT_DIR/proxy-9797.sh" --stop 2>/dev/null || true
	bash "$SCRIPT_DIR/proxy-9798.sh" --stop 2>/dev/null || true
	echo "All proxies stopped"
	exit 0
	;;
--status)
	echo "=== :9797 (Rust cache) ==="
	bash "$SCRIPT_DIR/proxy-9797.sh" --status 2>/dev/null || echo "DOWN"
	echo ""
	echo "=== :9798 (Rust token) ==="
	bash "$SCRIPT_DIR/proxy-9798.sh" --status 2>/dev/null || echo "DOWN"
	exit 0
	;;
"") ;;
*)
	echo "Unknown option: $1" >&2
	echo "Usage: $0 [--stop|--status]" >&2
	exit 1
	;;
esac

echo "Launching all Aphrodite proxies..."
echo ""

# :9797 - Rust cache
echo "► :9797 (Rust cache, in-memory CCR)"
bash "$SCRIPT_DIR/proxy-9797.sh" &
sleep 2

# :9798 - Rust token (tool relay)
echo "► :9798 (Rust token, tool relay)"
bash "$SCRIPT_DIR/proxy-9798.sh" &
sleep 2

echo ""
echo "All proxies launched. Check status with: $0 --status"
