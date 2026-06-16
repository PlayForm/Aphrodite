#!/usr/bin/env bash
set -euo pipefail

# build-monitor.sh
# Polls pane 17 wezterm buffer every 5s, parses cargo build status,
# writes .hermes/build-status.json for fix agents to consume.
# Agents: read .hermes/build-status.json instead of running cargo check.

STATUS_FILE="$(cd "$(dirname "$0")/.." && pwd)/.hermes/build-status.json"
mkdir -p "$(dirname "$STATUS_FILE")"

write_status() {
  local status="$1"      # ok | building | error
  local last_build="$2"  # ISO timestamp
  local version="$3"     # version string (empty string if none)
  local errors_json="$4" # JSON array string, e.g. '["err1","err2"]' or '[]'

  python3 -c "
import json, sys
status = json.loads('$status')
last_build = json.loads('$last_build')
errors = json.loads('$errors_json')
data = {'status': status, 'last_build': last_build, 'errors': errors}
if '$version':
    data['version'] = json.loads('$version')
with open('$STATUS_FILE', 'w') as f:
    json.dump(data, f, indent=2)
"
}

while true; do
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  # Read last 20 lines of pane 17
  buffer="$(wezterm cli get-text --pane-id 17 --start-line -20 2>&1 || true)"

  if [ -z "$buffer" ]; then
    # Pane might be gone temporarily - keep existing status
    sleep 5
    continue
  fi

  last20="$buffer"

  # Parse signals
  has_compiling="$(echo "$last20" | grep -c 'Compiling aphrodite' || true)"
  has_finished="$(echo "$last20" | grep -c 'Finished' || true)"
  has_running="$(echo "$last20" | grep -c 'Running' || true)"
  has_error="$(echo "$last20" | grep -cE 'error\[|could not compile' || true)"

  # Collect individual error messages (up to 10)
  errors=()
  while IFS= read -r line; do
    trimmed="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [ -n "$trimmed" ] && errors+=("$trimmed")
  done < <(echo "$last20" | grep -oE 'error\[.*?\]|could not compile.*' | head -10)

  if [ "$has_error" -gt 0 ]; then
    # Build has errors
    errors_json="["
    sep=""
    for e in "${errors[@]}"; do
      escaped="$(echo "$e" | python3 -c "import json,sys; print(json.dumps(sys.stdin.read().strip()))")"
      errors_json+="${sep}${escaped}"
      sep=", "
    done
    errors_json+="]"
    write_status '"error"' "\"$now\"" '""' "$errors_json"

  elif [ "$has_finished" -gt 0 ] && [ "$has_running" -gt 0 ]; then
    # Finished + Running = build succeeded and test is running
    write_status '"ok"' "\"$now\"" '"v0.5.61"' '[]'

  elif [ "$has_compiling" -gt 0 ]; then
    # Currently compiling without errors
    write_status '"building"' "\"$now\"" '""' '[]'

  else
    # No clear signal - preserve existing status if present
    if [ ! -f "$STATUS_FILE" ]; then
      write_status '"ok"' "\"$now\"" '""' '[]'
    fi
  fi

  sleep 5
done
