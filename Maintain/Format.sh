#!/usr/bin/env sh

#===============================================================================
# Format.sh — Format Prettier (TS/JS/JSON/MD/TOML/YAML).
#===============================================================================
#
# Usage:
#   sh Maintain/Format.sh               # Run Prettier
#   sh Maintain/Format.sh dos2unix      # Normalize line endings only
#   sh Maintain/Format.sh prettier      # Format with Prettier only
#
# Configuration:
#   .editorconfig      — Shared indent/newline rules
#   prettier.config.js — Prettier options and plugins
#   .prettierignore    — Paths excluded from Prettier formatting
#
#===============================================================================

\set -e

Current=$(cd -- "$(dirname -- "$0")" > /dev/null 2>&1 && pwd)

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

	if ! \command -v dos2unix > /dev/null 2>&1; then
		\echo "Error: dos2unix is not installed."
		\echo "  macOS:  brew install dos2unix"
		\echo "  Linux:  apt install dos2unix  /  dnf install dos2unix"
		\exit 1
	fi

	\cd "$Root"

	# Convert CRLF -> LF on every text file. dos2unix skips binary files
	# automatically. Exclude vendored, build output, and generated paths.
	#
	# shellcheck disable=SC2038
	\find . -type f \
		-not -path "*/vendor/*" \
		-not -path "*/node_modules/*" \
		-not -path "*/.git/*" \
		-not -path "*/Target/*" \
		-not -path "*/target/*" \
		-not -path "*/.fingerprint/*" \
		-not -path "*/incremental/*" \
		-not -path "*/deps/*" \
		| \xargs \dos2unix -q

	\echo ""
	\echo "Line ending conversion complete."
	\echo ""
}

FormatPrettier() {
	\echo "========================================"
	\echo "Format Prettier"
	\echo "========================================"
	\echo "Tooling: Prettier  (TS/JS/JSON/MD/TOML/YAML)"
	\echo "Config:  prettier.config.js"
	\echo "Ignore:  .prettierignore"
	\echo "========================================"
	\echo ""

	\cd "$Root"

	if [ -x "$Root/node_modules/.bin/prettier" ]; then
		"$Root/node_modules/.bin/prettier" --write . \
			--ignore-path ".prettierignore" || \true
	else
		\echo "Prettier not found in node_modules/.bin — skipping."
		\echo "Run: pnpm install"
	fi

	\echo ""
	\echo "Prettier formatting complete."
	\echo ""
}

#===============================================================================
# Main Command Router
#===============================================================================

case "${1:-}" in
	dos2unix)
		FormatLineEndings
		;;
	prettier)
		FormatPrettier
		;;
	"")
		FormatLineEndings
		FormatPrettier
		;;
	--help | -h)
		\echo "Usage: $0 [dos2unix|prettier]"
		\echo ""
		\echo "  dos2unix  Normalize line endings (CRLF -> LF) with dos2unix"
		\echo "  prettier  Format TS/JS/JSON/MD/TOML/YAML with Prettier"
		\echo "  (no arg)  Run both in order"
		;;
	*)
		\echo "Unknown target: $1"
		\echo "Use --help for usage information"
		\exit 1
		;;
esac
