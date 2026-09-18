#!/bin/bash
# hit-metrics.sh - generate traffic against both aphrodite proxies
# Usage: ./scripts/hit-metrics.sh [count=10] [delay=2]

COUNT="${1:-10}"
DELAY="${2:-2}"
CACHE="http://127.0.0.1:9797"
TOKEN="http://127.0.0.1:9798"

echo "Hitting aphrodite proxies ${COUNT}x with ${DELAY}s delay..."
echo "  Cache: $CACHE"
echo "  Token: $TOKEN"
echo "  Prometheus: http://localhost:9090"
echo ""

for i in $(seq 1 "$COUNT"); do
	# Hit /metrics on both (these go through the router, not proxy_handler)
	curl -s "$TOKEN/metrics" >/dev/null &
	curl -s "$CACHE/metrics" >/dev/null &

	# Hit chat completions (through proxy_handler → increments requests_total)
	curl -s -X POST "$TOKEN/v1/chat/completions" \
		-H "Content-Type: application/json" \
		-d '{"model":"test","messages":[{"role":"user","content":"ping"}]}' \
		--connect-timeout 5 --max-time 10 >/dev/null 2>&1 || true &
	curl -s -X POST "$CACHE/v1/chat/completions" \
		-H "Content-Type: application/json" \
		-d '{"model":"test","messages":[{"role":"user","content":"ping"}]}' \
		--connect-timeout 5 --max-time 10 >/dev/null 2>&1 || true &

	# Hit retrieve (exercises CCR path)
	curl -s "$TOKEN/retrieve?hash=test$i" >/dev/null &
	curl -s "$CACHE/retrieve?hash=test$i" >/dev/null &

	printf "  [%2d/%2d]" "$i" "$COUNT"
	sleep "$DELAY"
done

echo ""
echo "Done. Check metrics:"
echo "  curl -s $TOKEN/metrics | grep requests_total"
echo "  curl -s $CACHE/metrics | grep requests_total"
