#!/bin/bash
# enumerate-open-prs.sh <owner> [outfile]
# Writes every open PR in the org as OWNER/REPO<TAB>NUMBER lines (tab-separated).
set -u
OWNER="${1:?usage: enumerate-open-prs.sh <owner> [outfile]}"
OUT="${2:-/dev/stdout}"

total=$(gh api "search/issues?q=is:pr+is:open+org:$OWNER&per_page=1" --jq '.total_count')
pages=$(( (total + 99) / 100 ))
for p in $(seq 1 "$pages"); do
  gh api "search/issues?q=is:pr+is:open+org:$OWNER&per_page=100&page=$p" \
    --jq '.items[] | "\(.repository_url | sub(".*/repos/"; ""))\t\(.number)"'
done > "$OUT"

lines=$(wc -l < "$OUT" | tr -d ' ')
echo "total=$total listed=$lines"
[ "$lines" -eq "$total" ] || echo "WARNING: listed count != total (search truncation?)" >&2
