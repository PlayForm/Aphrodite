#!/bin/bash
# merge-close-sweep.sh <list-file> [log-file]
# list-file lines: OWNER/REPO<TAB>NUMBER (tab-separated; see enumerate-open-prs.sh)
# Merges each PR (squash + delete branch); closes with a comment ONLY on merge-conflict
# errors; logs and leaves open any other failure. Prints a SUMMARY line at the end.
set -u
LIST="${1:?usage: merge-close-sweep.sh <list-file> [log-file]}"
LOG="${2:-$(dirname "$LIST")/merge-close-sweep.log}"
: > "$LOG"
merged=0; closed=0; failed=0

while IFS=$'\t' read -r repo num; do
  out=$(gh pr merge "$num" --repo "$repo" --squash --delete-branch 2>&1)
  code=$?
  if [ $code -eq 0 ]; then
    echo "MERGED $repo#$num" | tee -a "$LOG"
    merged=$((merged+1))
  elif echo "$out" | grep -qi "merge conflict"; then
    c=$(gh pr close "$num" --repo "$repo" --comment "Closed: merge conflicts with the base branch." 2>&1)
    echo "CLOSED-CONFLICT $repo#$num :: $c" | tee -a "$LOG"
    closed=$((closed+1))
  else
    echo "FAILED $repo#$num :: $(echo "$out" | tail -1)" | tee -a "$LOG"
    failed=$((failed+1))
  fi
done < "$LIST"

echo "SUMMARY merged=$merged closed_conflict=$closed failed=$failed" | tee -a "$LOG"
