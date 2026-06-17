#!/bin/bash
# release-notes.sh — standardized aphrodite release notes generator
# Usage: ./scripts/release-notes.sh v0.5.122 "Short title" "Long description..."
# Outputs to stdout — pipe to: gh release create v0.5.122 --notes-file <(./scripts/release-notes.sh ...)
# Or: ./scripts/release-notes.sh v0.5.122 "title" "body..." > /tmp/notes.md && gh release create v0.5.122 -F /tmp/notes.md ~/.hermes/aphrodite/aphrodite
set -euo pipefail

TAG="${1:-}"
TITLE="${2:-}"
shift 2 2>/dev/null || true
BODY="${*:-}"

if [ -z "$TAG" ]; then
  echo "Usage: $0 vX.Y.Z 'Title' 'Body text...'" >&2
  exit 1
fi

# Extract previous tag for compare link
PREV=$(git tag --sort=-v:refname | grep -v "$TAG" | head -1)

cat <<EOF
**[Compare ${PREV}...${TAG}](https://github.com/PlayForm/Aphrodite/compare/${PREV}...${TAG})**

${BODY}
EOF
