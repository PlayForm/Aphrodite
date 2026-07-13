#!/bin/bash
# Monitor a wezterm pane's cargo watch output, write build-status.json every 5s
# Run via: WEZTERM_PANE_ID=17 hermes tool terminal "bash .hermes/build-watcher.sh" background=true

WEZTERM_PANE_ID="${WEZTERM_PANE_ID:-}"
STATE_DIR="$HOME/.hermes"
OUTPUT_FILE="$STATE_DIR/build-status.json"
mkdir -p "$STATE_DIR"

log() { echo "[$(date +%H:%M:%S)] $*"; }

write_status() {
	local status="$1" errs="$2"
	cat > "$OUTPUT_FILE" << JSONEOF
{"status":"$status","timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","errors":$(echo "$errs" | jq -R -s -c 'split("\n") | map(select(length > 0))')}
JSONEOF
}

get_buffer() {
	# Use hermes's mcp_wezterm_get_buffer via the running session's tool API
	# We'll use the hermes CLI directly since it can invoke MCP tools
	if [ -z "$WEZTERM_PANE_ID" ]; then
		return 1
	fi
	local result
	result=$(hermes tool mcp wezterm get_buffer --pane-id "$WEZTERM_PANE_ID" --lines 8 2> /dev/null)
	echo "$result"
}

# Initial status
write_status "idle" ""

if [ -z "$WEZTERM_PANE_ID" ]; then
	log "WEZTERM_PANE_ID not set - run as: WEZTERM_PANE_ID=<id> bash $0"
	exit 1
fi
log "Starting pane $WEZTERM_PANE_ID monitor (every 5s)"

while true; do
	buffer=$(get_buffer 2> /dev/null || true)

	if [ -z "$buffer" ]; then
		sleep 5
		continue
	fi

	# Extract the result value from JSON
	content=$(echo "$buffer" | python3 -c "
import sys, json
data = json.load(sys.stdin)
result = data.get('result', '')
print(result)
" 2> /dev/null || echo "$buffer")

	# Parse for patterns
	errors=""
	compiling=0
	running=0
	finished_ok=0

	while IFS= read -r line; do
		case "$line" in
			*"error"* | *"Error"* | *"ERROR"*)
				if [[ "$line" != *"INFO"* && "$line" != *"error:"*"listening"* ]]; then
					errors+="$line"$'\n'
				fi
				;;
			*"Compiling"*) compiling=1 ;;
			*"Running"*) running=1 ;;
			*"Finished"*"successfully"* | *"Finished \`dev\`"*) finished_ok=1 ;;
			*"warning"* | *"Warning"*)
				# warnings are non-fatal, track them
				;;
		esac
	done <<< "$content"

	if [ -n "$errors" ]; then
		write_status "error" "$errors"
		log "ERRORS detected"
	elif [ "$compiling" -eq 1 ] && [ "$finished_ok" -eq 0 ]; then
		write_status "compiling" ""
		log "Compiling..."
	elif [ "$running" -eq 1 ]; then
		write_status "running" ""
		log "Running"
	else
		write_status "idle" ""
	fi

	sleep 5
done
