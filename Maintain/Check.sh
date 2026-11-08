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
#   sh Maintain/Check.sh parity     # only the mirror-parity step
#
# Configuration:
#   .cargo/config.toml   - the ci-* aliases this script invokes; each value is
#                          the CI command with the leading `cargo ` removed
#   rust-toolchain.toml  - the nightly channel both CI and this machine resolve
#   deny.toml            - cargo-deny policy for the Deny job
#
# The run starts by updating the toolchain. The channel is floating by policy
# (rust-toolchain.toml `channel = "nightly"`, never a dated pin), so the local
# compiler is only honest if it is refreshed: a stale nightly is exactly the
# divergence that let `clippy::new_instead_of_clear` pass here and fail CI.
# `rustup toolchain install nightly --profile minimal --force` costs nothing when
# the channel is already current (rustup sees the unchanged manifest) and
# downloads the new compiler when it is not; `--no-self-update` keeps rustup from
# replacing its own binary mid-run. The step then probes `cargo fmt --version`
# and `cargo clippy --version` - both are hard CI gates - and records `rustc -Vv`
# and `cargo -V` in the log. Any failure aborts before a single gate runs: this
# script never continues on an older compiler than CI's.
#
# Every step below is one step of Check.yml, run in the workflow's own order:
# Rust job, then Test job, then Python job, then Deny job. The commands are the
# ci-* aliases, never inline copies - `cargo ci-check` IS the CI command.
# Maintain/CheckParity.py compares this script, .cargo/config.toml,
# rust-toolchain.toml and the workflow, and is itself a CI step in the Rust job.
#
# `--release` is deliberate: CI checks and tests in the release profile, and a
# debug-profile run can pass on code release rejects. The first cold release
# build costs minutes; afterwards the release target dir is warm.
#
# A step whose required tool is not installed FAILS this run. CI installs every
# tool it needs, so a local run without one cannot reproduce CI and must not
# report success - that is the whole point of a mirror. CHECK_OPTIONAL=1 is the
# explicit permissive mode: the step that needs the missing tool is skipped and
# named in the summary, INCOMPLETE is printed, and the exit status stays 0 for
# the steps that did run. Tool requirements are checked before their steps, so a
# missing tool fails fast instead of after the expensive Rust steps.
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

# RequireTool: the CI step's tool must exist.
#
# Returns 0 when the tool is present, and 1 when it is absent AND
# CHECK_OPTIONAL=1 - the caller then skips the step that needs it and names it in
# the summary. Without CHECK_OPTIONAL the run stops here, naming the tool and its
# install command: a silent skip is what this replaces, because it let a run
# report "All mirrored CI steps passed" while never executing the step.
RequireTool() {
	Label=$1
	Tool=$2
	Hint=$3

	if command -v "$Tool" > /dev/null 2>&1; then
		return 0
	fi

	if [ "${CHECK_OPTIONAL:-0}" = "1" ]; then
		\echo "→ $Label: '$Tool' is not installed ($Hint)."
		\echo "  CHECK_OPTIONAL=1: the step that needs it is skipped and named"
		\echo "  in the summary."
		\echo ""
		return 1
	fi

	\echo "========================================"
	\echo "$Label"
	\echo "========================================"
	\echo "Error: required tool '$Tool' is not installed."
	\echo "  Install it: $Hint"
	\echo "  CI installs every tool it needs, so this run cannot reproduce CI"
	\echo "  without it. Refusing to report success."
	\echo "  To run the remaining steps anyway: CHECK_OPTIONAL=1 $0"
	\echo "========================================"
	\echo ""
	exit 1
}

Should() {
	[ "$Wanted" = "all" ] || [ "$Wanted" = "$1" ]
}

#--- Toolchain (local equivalent of CI's dtolnay/rust-toolchain step) --------

# `rustup toolchain install` reads rust-toolchain.toml for the components
# (rustfmt, clippy) exactly as the CI action's `with: components:` does, so this
# installs the same compiler CI installs - no `+nightly`, no dated pin.
BootstrapToolchain() {
	if ! command -v rustup > /dev/null 2>&1; then
		\echo "Error: rustup is not on PATH - cannot update the toolchain."
		\echo "  Install rustup: https://rustup.rs"
		return 1
	fi
	\echo "→ rustup: $(command -v rustup)"

	\echo "→ Updating the nightly toolchain (channel: rust-toolchain.toml)…"
	if ! rustup toolchain install nightly --profile minimal --no-self-update --force; then
		\echo ""
		\echo "Error: rustup could not install/update the nightly toolchain."
		\echo "  Aborting: every gate below would run on an older compiler than CI's."
		return 1
	fi

	\echo ""
	\echo "→ Probes (both are hard CI gates):"
	if ! cargo fmt --version; then
		\echo "Error: cargo fmt is unavailable on the resolved toolchain."
		\echo "  CI's 'cargo fmt --check' step cannot be reproduced."
		return 1
	fi
	if ! cargo clippy --version; then
		\echo "Error: cargo clippy is unavailable on the resolved toolchain."
		\echo "  CI's 'cargo clippy' step cannot be reproduced."
		return 1
	fi

	\echo ""
	\echo "→ Compiler this run uses:"
	rustc -Vv
	cargo -V
}

RunToolchain() {
	cd "$Root"
	RunStep "toolchain" "dtolnay/rust-toolchain (setup action, mirrored locally)" BootstrapToolchain
}

#--- Precondition: mirror parity ---------------------------------------------

# The parity check runs before any CI step, for every target: a runner that
# mirrors a workflow it has drifted from is worse than no runner, because it
# reports green for gates CI no longer runs. Failures name the divergence.
# Since this is also a step of Check.yml (Rust job), it is not a local extra -
# CI enforces the same consistency on every push.
MirrorParity() {
	# Name the interpreter: python3 resolves differently between an interactive
	# shell and a script's PATH here, and the macOS system python3 (3.9) has no
	# tomllib. CheckParity.py carries a fallback parser, so either works - this
	# line makes a failure attributable.
	\echo "→ Interpreter: $(command -v python3) ($(python3 -c 'import sys; print(sys.version.split()[0])' 2> /dev/null || \echo unknown))"
	python3 Maintain/CheckParity.py
}

RunParity() {
	if [ -n "${CHECK_SKIP_PARITY:-}" ]; then
		\echo "Skipping mirror parity (CHECK_SKIP_PARITY is set)."
		\echo ""
		return 0
	fi
	cd "$Root"
	RunStep "mirror parity" "Rust / Mirror parity" MirrorParity
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
	# CI's `Install uv` step is a setup action, not a command; locally it is the
	# presence requirement for the Python FFI step. Checked first so a missing
	# tool fails in a second rather than after the release test build.
	HasUv=0
	if RequireTool "Install uv" uv "curl -LsSf https://astral.sh/uv/install.sh | sh"; then
		HasUv=1
	fi

	cd "$Root"
	RunStep "cargo test" "Test / cargo test" cargo ci-test

	cd "$Root"
	RunStep "cargo test (Maintain/dynamic_hooks)" "Test / cargo test (Maintain/dynamic_hooks)" cargo ci-hooks

	cd "$Root"
	RunStep "Build aphrodite-hermes dylib for FFI tests" "Test / Build aphrodite-hermes dylib for FFI tests" cargo ci-dylib

	if [ "$HasUv" = "1" ]; then
		cd "$Root"
		RunStep "Python FFI tests" "Test / Python FFI tests" PythonFFITests
	else
		SkipStep "Python FFI tests" "uv not installed (CHECK_OPTIONAL=1)"
	fi
}

PythonFFITests() {
	APHRODITE_REQUIRE_DYLIB=1 uv run --with pytest pytest tests/ -q
}

#--- Python job --------------------------------------------------------------

CheckPython() {
	HasRuff=0
	if RequireTool "ruff" ruff "uv tool install ruff"; then
		HasRuff=1
	fi

	HasNpx=0
	if RequireTool "pyright" npx "install Node.js (npx ships with it)"; then
		HasNpx=1
	fi

	cd "$Root"
	RunStep "Verify submodules" "Python / Verify submodules" VerifySubmodules

	if [ "$HasRuff" = "1" ]; then
		cd "$Root"
		RunStep "ruff" "Python / ruff-action (src: plugins/aphrodite/)" Ruff
	else
		SkipStep "ruff" "ruff not installed (CHECK_OPTIONAL=1)"
	fi

	if [ "$HasNpx" = "1" ]; then
		cd "$Root"
		RunStep "pyright" "Python / pyright" Pyright
	else
		SkipStep "pyright" "npx not installed (CHECK_OPTIONAL=1)"
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
		\echo "  parity  only the mirror-parity step"
		\echo "  (no arg) every CI step, in CI order"
		\echo ""
		\echo "Environment:"
		\echo "  CHECK_OPTIONAL=1    skip steps whose tool is missing; report INCOMPLETE"
		\echo "  CHECK_SKIP_PARITY=1 skip the mirror-parity step"
		exit 0
		;;
	*)
		\echo "Unknown target: $Wanted"
		\echo "Use --help for usage information"
		exit 1
		;;
esac

RunToolchain
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
	\echo ""
	\echo "INCOMPLETE: $Skipped CI step(s) did not run (CHECK_OPTIONAL=1)."
	\echo "  The steps above passed; the skipped ones are unproven locally."
	\echo ""
	exit 0
fi

\echo ""
\echo "All mirrored CI steps passed."
\echo ""
