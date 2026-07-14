#!/usr/bin/env bash
# docs-consistency-oneshot.sh - headless docs/version/link consistency pass via `hermes -z`.
#
# Runs Hermes in one-shot mode against THIS repo to verify (and, with --fix,
# repair) three recurring documentation-drift classes:
#   1. Version strings that claim a stale CURRENT version (vs correct historical
#      provenance like "fixed in vX.Y.Z", which must be preserved).
#   2. Markdown/badge links that point INTO a git submodule directory
#      (plugins/aphrodite, vendor/headroom, vendor/rtk) - these 404 on GitHub
#      because submodules render as commit gitlinks, not browseable trees.
#   3. Stale counts (tools/hooks/classifier types/ports) vs the actual code.
#
# The Aphrodite plugin (CCR compression) loads automatically in the Hermes
# session, so the agent's own tool output is compressed while it works - this
# script doubles as a real-world dogfooding run.
#
# Usage:
#   Maintain/scripts/ops/docs-consistency-oneshot.sh            # report only (read-only)
#   Maintain/scripts/ops/docs-consistency-oneshot.sh --fix      # apply edits, no commit
#   MODEL=anthropic/claude-sonnet-5 Maintain/scripts/ops/docs-consistency-oneshot.sh
#
# Requires: hermes on PATH with a configured provider. Never commits/tags/pushes.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$REPO_ROOT"

MODE="report on findings only (do NOT edit any file)"
if [[ "${1:-}" == "--fix" ]]; then
	MODE="apply the minimal fixes directly to the files (edit in place). Do NOT commit, tag, push, build, or rebuild."
fi

# Source of truth for the current version = the workspace crate version.
CUR_VER="$(grep -m1 '^version' crates/aphrodite/Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
USAGE_FILE="${USAGE_FILE:-/tmp/aphrodite-docs-oneshot-usage.json}"
MODEL_ARG=()
[[ -n "${MODEL:-}" ]] && MODEL_ARG=(--model "$MODEL")

read -r -d '' PROMPT <<EOF || true
You are running headless in the Aphrodite repo. Task: ${MODE}

Source of truth for the CURRENT version is crates/aphrodite/Cargo.toml = ${CUR_VER}.

JOB 1 - version currency. Bump ONLY stale current-version claims to ${CUR_VER}:
plugins/aphrodite/BINARY_VERSION (submodule file - note it separately), the
README release badge, and any health/stats JSON EXAMPLE showing "version":"...".
PRESERVE historical provenance verbatim: "fixed in vX", "introduced in vX",
"(vX)" origin tags, "since vX" - those state WHEN a feature landed and are
correct. Never blanket-replace a version string.

JOB 2 - submodule links. GitHub renders submodule dirs as commit gitlinks, so
any link into plugins/aphrodite, vendor/headroom, or vendor/rtk (relative OR
absolute github.com/PlayForm/Aphrodite/tree|blob/<ref>/...) 404s. Rewrite each
to the standalone repo:
  plugins/aphrodite/<p> -> https://github.com/PlayForm/Aphrodite-Hermes/blob/Current/<p>
  vendor/headroom/<p>   -> https://github.com/PlayForm/Headroom/blob/Current/<p>
  vendor/rtk/<p>        -> https://github.com/PlayForm/rtk/blob/Current/<p>
(all standalone PlayForm repos use "Current" as their default branch, not "main")
Link to the repo root when the target is the submodule directory itself.

JOB 3 - counts. Fix stale tool/hook/classifier-type/port claims ONLY when you
verify them against code first (crates/aphrodite-hermes/src/schemas.rs,
crates/aphrodite/src/config.rs, lib.rs). Otherwise leave and note.

Scope: README.md, docs/**, crates/*/README.md, Maintain/*.md, plugins/aphrodite
BINARY_VERSION. Do NOT touch target/, node_modules/, vendor/ internals, .plans/.
Preserve structure, voice, and emoji. Print a concise report grouped by JOB with
every change as old -> new (or "would change" in report mode).
EOF

echo "==> hermes -z docs consistency (${1:-report}) - version target ${CUR_VER}"
hermes -z "$PROMPT" --usage-file "$USAGE_FILE" "${MODEL_ARG[@]}"
echo
echo "==> usage report: $USAGE_FILE"
[[ -f "$USAGE_FILE" ]] && cat "$USAGE_FILE"
