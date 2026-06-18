#!/bin/bash
# prometheus.sh - start/stop Prometheus Docker for aphrodite metrics
# Usage: ./scripts/prometheus.sh [start|stop|status]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTAINER="aphrodite-prometheus"
CONFIG="$REPO_ROOT/prometheus.yml"

status() {
	if docker ps --filter "name=$CONTAINER" --format '{{.Status}}' | grep -q "Up"; then
		echo "Prometheus: running"
		echo "  UI:    http://localhost:9090"
		echo "  Cache: http://localhost:9090/targets?scrapePool=aphrodite-cache"
		echo "  Token: http://localhost:9090/targets?scrapePool=aphrodite-token"
		docker ps --filter "name=$CONTAINER" --format '  {{.Status}}'
	else
		echo "Prometheus: stopped"
	fi
}

start() {
	if docker ps --filter "name=$CONTAINER" --format '{{.Names}}' | grep -q "$CONTAINER"; then
		echo "Already running"
		status
		return
	fi
	echo "Starting Prometheus..."
	docker run -d --name "$CONTAINER" \
		-p 9090:9090 \
		-v "$CONFIG:/etc/prometheus/prometheus.yml:ro" \
		--add-host=host.docker.internal:host-gateway \
		prom/prometheus
	echo "Started. UI at http://localhost:9090"
}

stop() {
	echo "Stopping Prometheus..."
	docker stop "$CONTAINER" 2> /dev/null || true
	docker rm "$CONTAINER" 2> /dev/null || true
	echo "Stopped."
}

case "${1:-status}" in
	start) start ;;
	stop) stop ;;
	status) status ;;
	*) echo "Usage: $0 {start|stop|status}" ;;
esac
