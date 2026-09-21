#!/bin/sh
#===============================================================================
# Format.sh - Format shell, Prettier, and Rust source code.
#===============================================================================
#
# Usage:
#   sh Maintain/Format.sh               # Run all formatters
#   sh Maintain/Format.sh dos2unix      # Normalize line endings only
#   sh Maintain/Format.sh shell         # Format shell scripts only (prettier)
#   sh Maintain/Format.sh prettier      # Format Markdown/JSON/YAML only
#   sh Maintain/Format.sh rust          # Format Rust only
#
# Configuration:
#   .editorconfig  - Shared indent/newline rules (shfmt reads this)
#   .prettierrc    - Prettier options (prettier-plugin-sh handles .sh)
#   .prettierignore - Paths excluded from Prettier formatting
#   rustfmt.toml   - rustfmt options (nightly, edition 2024)
#
# The Rust pass ends with the same gate CI runs (`cargo fmt --all -- --check`),
# so a formatted tree is always cargo-check compliant.
#
# TOML is handled by taplo/even-better-toml, Python by ruff - neither is
# invoked from here; see .github/workflows/Check.yml.
#===============================================================================

set -e

Current=$(cd -- "$(dirname -- "$0")" >/dev/null 2>&1 && pwd)
Root="$Current/.."

#===============================================================================
# Format Functions
#===============================================================================

FormatLineEndings() {
	\echo "========================================"
	\echo "Format Line Endings"
	\echo "========================================"
	\echo "Tooling: dos2unix"
	\echo "========================================"
	\echo ""

	if ! command -v dos2unix >/dev/null 2>&1; then
		\echo "Error: dos2unix is not installed."
		\echo "  macOS:  brew install dos2unix"
		\echo "  Linux:  apt install dos2unix  /  dnf install dos2unix"
		exit 1
	fi

	cd "$Root"

	# Convert CRLF -> LF on every text file. dos2unix skips binary files
	# automatically. `vendor/` and `plugins/` are separate git submodules
	# (PlayForm/Headroom, PlayForm/rtk, PlayForm/Aphrodite-Hermes) that may
	# legitimately check out as CRLF on this machine (core.autocrlf) as
	# their own normal state - do NOT bulk-convert an entire submodule's
	# working tree from here. If a specific vendored file's line endings
	# break local tooling (as happened once with
	# vendor/headroom/crates/headroom-core/Cargo.toml), fix that one file
	# by hand inside the submodule, scoped to just that file.
	# `profiles/` is gitignored/generated, so there is nothing to fix there.
	#
	# shellcheck disable=SC2038
	find . -type f \
		-not -path "*/vendor/*" \
		-not -path "*/plugins/*" \
		-not -path "*/profiles/*" \
		-not -path "*/.hermes/*" \
		-not -path "*/target/*" \
		-not -path "*/node_modules/*" \
		-not -path "*/.git/*" \
		-not -path "*/.fingerprint/*" \
		-not -path "*/incremental/*" \
		-not -path "*/deps/*" \
		-not -path "*/.turbo/*" \
		-not -path "*/.cache/*" \
		-not -path "*/Generated/*" \
		-not -path "*/.generated/*" \
		-not -path "*/gen/*" \
		-not -path "*/bin/*" |
		xargs dos2unix -q

	\echo ""
	\echo "Line ending conversion complete."
	\echo ""
}

FormatShell() {
	\echo "========================================"
	\echo "Format Shell"
	\echo "========================================"
	\echo "Tooling: Prettier (prettier-plugin-sh)"
	\echo "Ignore:  .prettierignore"
	\echo "========================================"
	\echo ""

	cd "$Root"

	\echo "→ Installing dependencies…"
	pnpm install --frozen-lockfile 2>/dev/null || pnpm install

	\echo "→ Running Prettier on shell scripts…"
	pnpm exec prettier --write \
		--ignore-path .prettierignore \
		"**/*.sh"

	\echo ""
	\echo "Shell formatting complete."
	\echo ""
}

FormatPrettier() {
	\echo "========================================"
	\echo "Format Prettier"
	\echo "========================================"
	\echo "Tooling: Prettier (md, json, yaml)"
	\echo "Ignore:  .prettierignore"
	\echo "========================================"
	\echo ""

	cd "$Root"

	\echo "→ Installing dependencies…"
	pnpm install --frozen-lockfile 2>/dev/null || pnpm install

	\echo "→ Running Prettier (md, json, yaml)…"
	pnpm exec prettier --write \
		--ignore-path .prettierignore \
		"**/*.md" \
		"**/*.json" \
		"**/*.yml" \
		"**/*.yaml"

	\echo ""
	\echo "Prettier formatting complete."
	\echo ""
}

FormatRust() {
	\echo "========================================"
	\echo "Format Rust"
	\echo "========================================"
	\echo "Tooling: cargo +nightly fmt  (workspace module tree, first)"
	\echo "         rustfmt direct pass (orphan files, second)"
	\echo "Config:  rustfmt.toml"
	\echo "========================================"
	\echo ""

	cd "$Root"

	# Pass 1: cargo fmt - formats every .rs file reachable via `mod`
	# declarations from crates/aphrodite and crates/aphrodite-hermes.
	# `vendor/headroom` and `vendor/rtk` are separate Cargo workspace roots
	# (their own [workspace] table) - cargo fmt here never touches them.
	cargo +nightly fmt --all

	# Pass 2: direct rustfmt - catches any .rs files that are NOT part of
	# this workspace's module graph (orphan files, planned modules not yet
	# wired into a crate root). Never touches vendor/ or plugins/ - those
	# are separate submodules with their own formatting.
	#
	# shellcheck disable=SC2038
	# shellcheck disable=SC2016
	find . -name "*.rs" \
		-not -path "*/vendor/*" \
		-not -path "*/plugins/*" \
		-not -path "*/profiles/*" \
		-not -path "*/.hermes/*" \
		-not -path "*/target/*" \
		-not -path "*/node_modules/*" \
		-not -path "*/.git/*" \
		-not -path "*/.fingerprint/*" \
		-not -path "*/incremental/*" \
		-not -path "*/deps/*" \
		-not -path "*/Generated/*" \
		-not -path "*/.generated/*" \
		-not -path "*/gen/*" |
		xargs -I {} sh -c \
			'rustup run nightly rustfmt --config-path rustfmt.toml "$1" 2>/dev/null || true' \
			-- {}

	# Compliance: the exact gate CI runs in Check.yml. A formatted tree must
	# pass `cargo fmt --all -- --check` - fail loudly if it does not.
	\echo "→ Verifying cargo fmt --all -- --check…"
	cargo +nightly fmt --all -- --check

	\echo ""
	\echo "Rust formatting complete (cargo fmt check passed)."
	\echo ""
}

#===============================================================================
# Main Command Router
#===============================================================================

case "${1:-}" in
dos2unix)
	FormatLineEndings
	;;
shell)
	FormatShell
	;;
prettier)
	FormatPrettier
	;;
rust)
	FormatRust
	;;
"")
	FormatLineEndings
	FormatShell
	FormatPrettier
	FormatRust
	\echo "→ Format complete."
	;;
--help | -h)
	\echo "Usage: $0 [dos2unix|shell|prettier|rust]"
	\echo ""
	\echo "  dos2unix  Normalize line endings (CRLF -> LF) with dos2unix"
	\echo "  shell     Format shell scripts with Prettier (prettier-plugin-sh)"
	\echo "  prettier  Format Markdown/JSON/YAML with Prettier"
	\echo "  rust      Format Rust with rustfmt (nightly)"
	\echo "  (no arg)  Run all four in order"
	;;
*)
	\echo "Unknown target: $1"
	\echo "Use --help for usage information"
	exit 1
	;;
esac
