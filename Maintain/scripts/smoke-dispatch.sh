#!/bin/bash
# Parallel worker smoke test - dispatches N workers via poll_worker.py
# Usage: bash Maintain/scripts/smoke-dispatch.sh
#
# Requires: Hermes Agent installed (hermes-agent venv at ~/.hermes/hermes-agent/venv)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HERMES_VENV="${HERMES_VENV:-$HOME/.hermes/hermes-agent/venv}"
VENV_PYTHON="$HERMES_VENV/bin/python3"
POLLER="$VENV_PYTHON $PROJECT/Maintain/scripts/poll_worker.py"
RESULTS_DIR="${RESULTS_DIR:-$HOME/.hermes/temporary/smoke-$(date +%s)}"
mkdir -p "$RESULTS_DIR"

# Workers 1-3: Stats + compress (tests token cache)
for i in 1 2 3; do
	# shellcheck disable=SC2046
	TMP="$(mktemp ~/.hermes/temporary/poll-$(date +%s)-$RANDOM.XXXXXX.md)"
	cat >"$TMP" <<'PROMPT'
aphrodite_stats then aphrodite_compress type=text content="setup smoke test" then aphrodite_stats
PROMPT
	$POLLER "$TMP" --provider aphrodite-token --model deepseek-v4-flash --toolsets terminal,file >"$RESULTS_DIR/w${i}.log" 2>&1 &
	echo "Worker $i dispatched (PID $!)"
done

# Workers 4-6: Read + compress (tests read_file + cache hit)
for i in 4 5 6; do
	# shellcheck disable=SC2046
	TMP="$(mktemp ~/.hermes/temporary/poll-$(date +%s)-$RANDOM.XXXXXX.md)"
	cat >"$TMP" <<'PROMPT'
aphrodite_stats then read_file path="Cargo.toml" limit=10 then aphrodite_compress type=code content="test compression" then aphrodite_stats
PROMPT
	$POLLER "$TMP" --provider aphrodite-token --model deepseek-v4-flash --toolsets terminal,file >"$RESULTS_DIR/w${i}.log" 2>&1 &
	echo "Worker $i dispatched (PID $!)"
done

echo "ALL DISPATCHED - results in $RESULTS_DIR/"
wait
echo "ALL COMPLETE"
echo ""
echo "=== Results ==="
for f in "$RESULTS_DIR"/w*.log; do
	NAME=$(basename "$f")
	# shellcheck disable=SC2034
	EXIT=$(tail -1 "$f" 2>/dev/null | grep -c "exit_code: 0" || echo "?")
	echo "$NAME: $(wc -l <"$f" | tr -d ' ') lines"
done
