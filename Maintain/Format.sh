#!/bin/sh
#===============================================================================
# Format.sh - Format shell, Prettier, and Rust source code.
#===============================================================================
#
# Usage:
#   sh Maintain/Format.sh               # Run all formatters
#   sh Maintain/Format.sh dos2unix      # Normalize line endings only
#   sh Maintain/Format.sh shell         # Format shell scripts only
#   sh Maintain/Format.sh prettier      # Format Markdown/JSON/YAML only
#   sh Maintain/Format.sh rust          # Format Rust only
#
# Configuration:
#   .editorconfig  - Shared indent/newline rules (shfmt reads this)
#   rustfmt.toml   - rustfmt options (nightly, edition 2021)
#   .prettierignore - Paths excluded from Prettier formatting
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
	echo "========================================"
	echo "Format Line Endings"
	echo "========================================"
	echo "Tooling: dos2unix"
	echo "========================================"
	echo ""

	if ! command -v dos2unix >/dev/null 2>&1; then
		echo "Error: dos2unix is not installed."
		echo "  macOS:  brew install dos2unix"
		echo "  Linux:  apt install dos2unix  /  dnf install dos2unix"
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
		-not -path "*/bin/*" \
		| xargs dos2unix -q

	echo ""
	echo "Line ending conversion complete."
	echo ""
}

FormatShell() {
	echo "========================================"
	echo "Format Shell"
	echo "========================================"
	echo "Tooling: shfmt"
	echo "Config:  .editorconfig (tabs, indent=4)"
	echo "========================================"
	echo ""

	if ! command -v shfmt >/dev/null 2>&1; then
		echo "Error: shfmt is not installed."
		echo "  macOS:  brew install shfmt"
		echo "  Linux:  apt install shfmt  /  go install mvdan.cc/sh/v3/cmd/shfmt@latest"
		echo "  https://github.com/mvdan/sh"
		exit 1
	fi

	cd "$Root"

	# shfmt reads .editorconfig for indent style/size automatically.
	# `vendor/` and `plugins/` are separate git submodules with their own
	# formatting - never reformat their style from here. `profiles/` is
	# gitignored/generated; `.hermes/` is runtime data.
	#
	# shellcheck disable=SC2038
	find . -name "*.sh" \
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
		-not -path "*/bin/*" \
		| xargs shfmt -w

	echo ""
	echo "Shell formatting complete."
	echo ""
}

FormatPrettier() {
	echo "========================================"
	echo "Format Prettier"
	echo "========================================"
	echo "Tooling: Prettier (md, json, yaml)"
	echo "Ignore:  .prettierignore"
	echo "========================================"
	echo ""

	cd "$Root"

	echo "→ Installing dependencies…"
	pnpm install --frozen-lockfile 2>/dev/null || pnpm install

	echo "→ Running Prettier (md, json, yaml)…"
	pnpm exec prettier --write \
		--ignore-path .prettierignore \
		"**/*.md" \
		"**/*.json" \
		"**/*.yml" \
		"**/*.yaml" \
		"*.md" \
		"*.json" \
		"*.yml" \
		"*.yaml"

	echo ""
	echo "Prettier formatting complete."
	echo ""
}

FormatRust() {
	echo "========================================"
	echo "Format Rust"
	echo "========================================"
	echo "Tooling: Format/Rust.py      (blank lines, first)"
	echo "         cargo +nightly fmt  (workspace module tree, second)"
	echo "         rustfmt direct pass (orphan files, third)"
	echo "Config:  rustfmt.toml"
	echo "========================================"
	echo ""

	cd "$Root"

	# Pass 1: blank-line formatter - inserts blank lines after statement and
	# block boundaries. Runs first so that rustfmt can normalize the result.
	python3 "$Current/Format/Rust.py" --All

	# Pass 2: cargo fmt - formats every .rs file reachable via `mod`
	# declarations from crates/aphrodite and crates/aphrodite-hermes.
	# `vendor/headroom` and `vendor/rtk` are separate Cargo workspace roots
	# (their own [workspace] table) - cargo fmt here never touches them.
	cargo +nightly fmt

	# Pass 3: direct rustfmt - catches any .rs files that are NOT part of
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
		-not -path "*/gen/*" \
		| xargs -I {} sh -c \
			'rustup run nightly rustfmt --config-path rustfmt.toml "$1" 2>/dev/null || true' \
			-- {}

	echo ""
	echo "Rust formatting complete."
	echo ""
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
		echo "→ Format complete."
		;;
	--help | -h)
		echo "Usage: $0 [dos2unix|shell|prettier|rust]"
		echo ""
		echo "  dos2unix  Normalize line endings (CRLF -> LF) with dos2unix"
		echo "  shell     Format shell scripts with shfmt"
		echo "  prettier  Format Markdown/JSON/YAML with Prettier"
		echo "  rust      Format Rust with rustfmt (nightly) + Rust.py"
		echo "  (no arg)  Run all four in order"
		;;
	*)
		echo "Unknown target: $1"
		echo "Use --help for usage information"
		exit 1
		;;
esac
