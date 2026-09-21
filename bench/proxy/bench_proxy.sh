#!/usr/bin/env bash
# End-to-end proxy benchmark wrapper. The python driver spawns the proxy on
# override ports 19797/19798 and cleans up after itself; this wrapper adds a
# belt-and-braces trap that kills anything still bound to the bench ports.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
BINARY="$REPO/target/release/aphrodite"

cleanup() {
	for port in 19797 19798; do
		local pids
		pids=$(lsof -ti tcp:"$port" 2>/dev/null || true)
		[ -n "$pids" ] && kill $pids 2>/dev/null || true
	done
}
trap cleanup EXIT INT TERM

if [ ! -x "$BINARY" ]; then
	\echo "missing $BINARY - run: cargo build --release -p aphrodite" >&2
	exit 1
fi

exec_python() { python3 "$HERE/bench_proxy.py" "$@"; }
exec_python "$@"
