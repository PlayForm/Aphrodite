#!/bin/bash
# Monitor pane 17 cargo watch, write build-status.json every 5s
# Run via: hermes tool terminal "bash .hermes/build-watcher.sh" background=true

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
	local result
	result=$(hermes tool mcp wezterm get_buffer --pane-id 17 --lines 8 2> /dev/null)
	echo "$result"
}

# Initial status
write_status "idle" ""

log "Starting pane 17 monitor (every 5s)"

while true; do
	buffer=$(mcp_wezterm_get_buffer 2> /dev/null || true)

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
