#!/usr/bin/env bash
#===============================================================================
# Check.sh - Local mirror of .github/workflows/Check.yml.
#===============================================================================
#
# Usage:
#   sh Maintain/Check.sh            # every CI step, in CI order
#   sh Maintain/Check.sh rust       # Rust job only (check, clippy, fmt)
#   sh Maintain/Check.sh test       # Test job only (test, hooks, dylib, pytest)
#   sh Maintain/Check.sh python     # Python job only (submodules, ruff, pyright)
#   sh Maintain/Check.sh deny       # Deny job only (cargo deny)
#   sh Maintain/Check.sh parity     # only the mirror-parity precondition
#
# Configuration:
#   .cargo/config.toml   - the ci-* aliases this script invokes; each value is
#                          the CI command with the leading `cargo ` removed
#   rust-toolchain.toml  - the dated nightly both CI and this machine resolve
#   deny.toml            - cargo-deny policy for the Deny job
#
# Every step below is one step of Check.yml, run in the workflow's own order:
# Rust job, then Test job, then Python job, then Deny job. The commands are the
# ci-* aliases, never inline copies - `cargo ci-check` IS the CI command.
# Maintain/CheckParity.py parses both files and fails when they diverge, so a
# step added to CI cannot go unmirrored here.
#
# `--release` is deliberate: CI checks and tests in the release profile, and a
# debug-profile run can pass on code release rejects. The first cold release
# build costs minutes; afterwards the release target dir is warm.
#
# Steps whose tool is not installed on this machine are SKIPPED with a warning
# and named in the summary - a missing local tool is not a CI failure. Set
# CHECK_STRICT=1 to turn any skip into a failure (what CI does, since CI
# installs every tool it needs).
#
# Known host limitation: the `cargo ci-test` step is red on this Mac and green
# on CI, for a reason that is neither this mirror nor the store code.
# `in_memory_get_refreshes_idle_ttl`
# (vendor/headroom/crates/headroom-core/tests/ccr_backends.rs) touches an entry
# every 60 ms inside a 120 ms idle window; thread::sleep(60ms) on this host
# actually returns after ~134 ms (measured), so the entry is already past its
# window at the first check. That is a portability bug in the vendored test.
# Do not "fix" it by dropping the step - CI runs the same command.
#
# Formatting is NOT part of this script: `cargo ci-fmt` only verifies. Run
# Maintain/Format.sh to format, which then calls this script to check.
#===============================================================================

set -e

Current=$(cd -- "$(dirname -- "$0")" > /dev/null 2>&1 && pwd)
Root="$Current/.."

Wanted="${1:-all}"

Steps=0
Skipped=0
SkippedList=""

#===============================================================================
# Check Functions
#===============================================================================

RunStep() {
	Label=$1
	CiStep=$2
	shift 2

	\echo "========================================"
	\echo "$Label"
	\echo "========================================"
	\echo "CI step: $CiStep"
	\echo "Command: $*"
	\echo "========================================"
	\echo ""

	Steps=$((Steps + 1))

	if "$@"; then
		\echo ""
		\echo "→ $Label: PASS"
		\echo ""
		return 0
	fi

	\echo ""
	\echo "Error: $Label failed (CI step: $CiStep)."
	\echo "  Command: $*"
	\echo "  This is the same command CI runs - fix it before pushing."
	\echo ""
	exit 1
}

SkipStep() {
	Label=$1
	Why=$2

	Steps=$((Steps + 1))
	Skipped=$((Skipped + 1))
	SkippedList="$SkippedList
  - $Label ($Why)"

	\echo "========================================"
	\echo "$Label"
	\echo "========================================"
	\echo "SKIPPED: $Why"
	\echo "========================================"
	\echo ""
}

Should() {
	[ "$Wanted" = "all" ] || [ "$Wanted" = "$1" ]
}

#--- Precondition (not a CI step) --------------------------------------------

# The parity check runs before any CI step, for every target: a runner that
# mirrors a workflow it has drifted from is worse than no runner, because it
# reports green for gates CI no longer runs. Failures name the divergence.
MirrorParity() {
	python3 Maintain/CheckParity.py
}

RunParity() {
	if [ -n "${CHECK_SKIP_PARITY:-}" ]; then
		\echo "Skipping mirror parity (CHECK_SKIP_PARITY is set)."
		\echo ""
		return 0
	fi
	cd "$Root"
	RunStep "mirror parity" "precondition (not a CI step)" MirrorParity
}

#--- Rust job ----------------------------------------------------------------

CheckRust() {
	cd "$Root"
	RunStep "cargo check" "Rust / cargo check" cargo ci-check

	cd "$Root"
	RunStep "cargo clippy" "Rust / cargo clippy" cargo ci-clippy

	cd "$Root"
	RunStep "cargo fmt --check" "Rust / cargo fmt --check" cargo ci-fmt
}

#--- Test job ----------------------------------------------------------------

CheckTest() {
	cd "$Root"
	RunStep "cargo test" "Test / cargo test" cargo ci-test

	cd "$Root"
	RunStep "cargo test (Maintain/dynamic_hooks)" "Test / cargo test (Maintain/dynamic_hooks)" cargo ci-hooks

	# CI's `Install uv` step is a setup action, not a command; locally it is a
	# presence check for the step that needs it.
	if ! command -v uv > /dev/null 2>&1; then
		SkipStep "Install uv" "uv not installed (curl -LsSf https://astral.sh/uv/install.sh | sh)"
	fi

	cd "$Root"
	RunStep "Build aphrodite-hermes dylib for FFI tests" "Test / Build aphrodite-hermes dylib for FFI tests" cargo ci-dylib

	if command -v uv > /dev/null 2>&1; then
		cd "$Root"
		RunStep "Python FFI tests" "Test / Python FFI tests" PythonFFITests
	else
		SkipStep "Python FFI tests" "uv not installed"
	fi
}

PythonFFITests() {
	APHRODITE_REQUIRE_DYLIB=1 uv run --with pytest pytest tests/ -q
}

#--- Python job --------------------------------------------------------------

CheckPython() {
	cd "$Root"
	RunStep "Verify submodules" "Python / Verify submodules" VerifySubmodules

	if command -v ruff > /dev/null 2>&1; then
		cd "$Root"
		RunStep "ruff" "Python / ruff-action (src: plugins/aphrodite/)" Ruff
	else
		SkipStep "ruff" "ruff not installed (uv tool install ruff)"
	fi

	if command -v npx > /dev/null 2>&1; then
		cd "$Root"
		RunStep "pyright" "Python / pyright" Pyright
	else
		SkipStep "pyright" "npx not installed (Node.js required)"
	fi
}

# CI's own step cannot fail: `ls plugins/aphrodite/plugin.yaml || echo MISSING`
# echoes and exits 0 either way. This mirror fails on the same condition, so a
# missing submodule is actually caught locally.
VerifySubmodules() {
	git submodule status
	test -f plugins/aphrodite/plugin.yaml
}

Ruff() {
	ruff check plugins/aphrodite/
}

# `--yes` is local-only: CI has no TTY, so npx installs pyright@1.1.411 without
# prompting there. The version and target directory are CI's.
Pyright() {
	cd "$Root/plugins/aphrodite"
	npx --yes pyright@1.1.411 .
}

#--- Deny job ----------------------------------------------------------------

CheckDeny() {
	cd "$Root"
	RunStep "cargo deny" "Deny / cargo deny" cargo ci-deny
}

#===============================================================================
# Main Command Router
#===============================================================================

case "$Wanted" in
	all | rust | test | python | deny | parity)
		;;
	--help | -h)
		\echo "Usage: $0 [rust|test|python|deny|parity]"
		\echo ""
		\echo "  rust    cargo check, cargo clippy, cargo fmt --check"
		\echo "  test    cargo test, dynamic_hooks tests, dylib build, pytest"
		\echo "  python  submodule verify, ruff, pyright"
		\echo "  deny    cargo deny check"
		\echo "  parity  only the mirror-parity precondition"
		\echo "  (no arg) every CI step, in CI order"
		\echo ""
		\echo "Environment:"
		\echo "  CHECK_STRICT=1      fail instead of skipping a step whose tool is absent"
		\echo "  CHECK_SKIP_PARITY=1 skip the mirror-parity precondition"
		exit 0
		;;
	*)
		\echo "Unknown target: $Wanted"
		\echo "Use --help for usage information"
		exit 1
		;;
esac

RunParity

if Should rust; then
	CheckRust
fi

if Should test; then
	CheckTest
fi

if Should python; then
	CheckPython
fi

if Should deny; then
	CheckDeny
fi

\echo "========================================"
\echo "Check Summary"
\echo "========================================"
\echo "Steps:   $Steps"
\echo "Skipped: $Skipped"

if [ "$Skipped" -gt 0 ]; then
	\echo "$SkippedList"
	if [ "${CHECK_STRICT:-0}" = "1" ]; then
		\echo ""
		\echo "Error: CHECK_STRICT=1 and $Skipped step(s) were skipped."
		exit 1
	fi
fi

\echo ""
\echo "All mirrored CI steps passed."
\echo ""
