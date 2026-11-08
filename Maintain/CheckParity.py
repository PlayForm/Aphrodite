#!/usr/bin/env python3
"""Parity checker: .github/workflows/Check.yml against the local mirror.

The local gates are only worth having if they are the SAME gates. This script
reads the workflow, reads the tracked config that mirrors it, and fails when the
two disagree - so a step added to CI cannot silently go unmirrored, an alias
cannot drift away from the command it claims to copy, and a mirrored shell
function cannot quietly change the command it runs.

What is compared:

1. **Step inventory.** Every step of every job in Check.yml must be known to
   ``SPEC`` below, in the workflow's own order. An appended ``- name: ...`` +
   ``run: cargo ...`` step therefore fails as unknown.
2. **Alias text.** For every cargo step, the value of the alias in
   ``.cargo/config.toml`` must equal the CI command with the leading
   ``cargo `` removed - byte for byte, flags included. Steps that run a GitHub
   action are expanded from that action's declared inputs (see the ACTION_*
   constants): ``cargo-deny-action`` with ``command: check`` becomes
   ``deny --all-features check``, ``ruff-action`` with ``src: X`` becomes
   ``ruff check X``.
3. **Alias coverage and runner order.** Every ``ci-*`` alias defined in
   ``.cargo/config.toml`` must be used by a step, and every alias a step names
   must exist. The runner side is parsed, not grepped: the ordered list of
   ``RunStep ... cargo ci-<name>`` invocations in ``Maintain/Check.sh`` must
   equal the CI step order. A comment that mentions an alias, or a function that
   runs a command nobody calls, no longer satisfies this.
4. **Mirrored script bodies.** For each step mirrored by a shell function, the
   command lines *inside that function* are compared with the CI step's own
   commands (its ``run:`` block, or the action expansion). Comments, ``cd``
   lines, ``echo`` lines and shell control flow are ignored; leading
   ``VAR=value`` prefixes are compared against the step's ``env:`` block.
   A deliberate difference must be declared in ``DIVERGENCE`` as a one-to-one
   substitution with a reason - and a substitution whose CI command no longer
   exists fails as stale, so neither side can drift silently.
5. **Toolchain.** Every ``dtolnay/rust-toolchain`` step must pass the same
   channel and components that ``rust-toolchain.toml`` pins. A floating channel
   on either side re-opens the local-vs-CI lint divergence this pins shut.

What this does NOT prove, stated so the guarantee is not overread: the runner's
``Should`` guards (that each branch selects the right job), the compiler rustup
resolves at run time, and an action's internals beyond the inputs declared here.
It proves the command and config inventory is consistent - which
is what a drift guard can prove.

Usage:
    python3 Maintain/CheckParity.py
    python3 Maintain/CheckParity.py --verbose          # print every command pair
    python3 Maintain/CheckParity.py --workflow <path>  # check a scratch copy

Exit codes: 0 when the inventory is consistent, 1 on any divergence, 2 on a
parse or usage error.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11, e.g. macOS system python3 3.9
    tomllib = None  # type: ignore[assignment]

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "Check.yml"
DEFAULT_CARGO_CONFIG = REPO_ROOT / ".cargo" / "config.toml"
DEFAULT_TOOLCHAIN = REPO_ROOT / "rust-toolchain.toml"
DEFAULT_SCRIPT = REPO_ROOT / "Maintain" / "Check.sh"

INFRA = ("infra", "")

# The declared mirror. Keys are (kind, identifier) as produced by step_key():
# ("name", <step name>) for named steps, ("uses", <action without @ref>) for
# unnamed action steps. Values say where the local equivalent lives:
#   ("alias", "<cargo alias>")  - an [alias] entry in .cargo/config.toml
#   ("script", "<FunctionName>") - a function in Maintain/Check.sh
#   INFRA                        - checkout / cache / toolchain setup; no
#                                  command of its own to mirror
SPEC = {
    "Rust": [
        (("uses", "actions/checkout"), INFRA),
        (("name", "Mirror parity"), ("script", "MirrorParity")),
        (("uses", "dtolnay/rust-toolchain"), INFRA),
        (("uses", "actions/cache"), INFRA),
        (("name", "cargo check"), ("alias", "ci-check")),
        (("name", "cargo clippy"), ("alias", "ci-clippy")),
        (("name", "cargo fmt --check"), ("alias", "ci-fmt")),
    ],
    "Test": [
        (("uses", "actions/checkout"), INFRA),
        (("uses", "dtolnay/rust-toolchain"), INFRA),
        (("uses", "actions/cache"), INFRA),
        (("name", "cargo test"), ("alias", "ci-test")),
        (("name", "cargo test (Maintain/dynamic_hooks)"), ("alias", "ci-hooks")),
        (("uses", "astral-sh/setup-uv"), INFRA),
        (("name", "Build aphrodite-hermes dylib for FFI tests"), ("alias", "ci-dylib")),
        (("name", "Python FFI tests"), ("script", "PythonFFITests")),
    ],
    "Python": [
        (("uses", "actions/checkout"), INFRA),
        (("name", "Verify submodules"), ("script", "VerifySubmodules")),
        (("uses", "astral-sh/ruff-action"), ("script", "Ruff")),
        (("name", "pyright"), ("script", "Pyright")),
    ],
    "Deny": [
        (("uses", "actions/checkout"), INFRA),
        (("uses", "EmbarkStudios/cargo-deny-action"), ("alias", "ci-deny")),
    ],
}

# Actions that only prepare the runner; their mirror is the local tool being
# installed, which Maintain/Check.sh requires before the dependent step.
INFRA_ACTIONS = {
    "actions/checkout",
    "actions/cache",
    "dtolnay/rust-toolchain",
    "astral-sh/setup-uv",
}

TOOLCHAIN_ACTION = "dtolnay/rust-toolchain"
DENY_ACTION = "EmbarkStudios/cargo-deny-action"
RUFF_ACTION = "astral-sh/ruff-action"

# Action inputs that turn an action step into a command. Each default is the
# value declared in that action's own action.yml at the ref Check.yml pins.
#
# cargo-deny-action v2.1.1 declares `arguments` default "--all-features" and its
# entrypoint appends `--log-level <log-level> --manifest-path <manifest-path>
# <arguments> <command>`; the observed argv in the failing job's log was
#   cargo deny --log-level warn --manifest-path ./Cargo.toml --all-features check
# `warn` and `./Cargo.toml` are cargo-deny's own defaults, so only the flag that
# changes the audited dependency graph appears in the alias.
DENY_DEFAULT_ARGUMENTS = "--all-features"
DENY_DEFAULT_COMMAND = "check"
# ruff-action v4.1.0 declares `args` default "check" and runs `ruff <args> <src>`.
RUFF_DEFAULT_ARGS = "check"
RUFF_DEFAULT_SRC = "./"

# Declared divergences: the ONE place a mirrored function may differ from CI.
# Each entry replaces an exact CI command with the exact local command and must
# say why. A replacement key the workflow no longer produces is reported as
# stale, so a divergence cannot outlive the CI line it was written for.
DIVERGENCE = {
    "VerifySubmodules": {
        "reason": (
            "CI's `ls plugins/aphrodite/plugin.yaml || echo MISSING` exits 0 whether "
            "or not the file exists, so it can never fail; the mirror fails on the "
            "same condition instead of echoing about it"
        ),
        "replace": {
            'ls plugins/aphrodite/plugin.yaml || echo "MISSING: plugins/aphrodite not checked out"': (
                "test -f plugins/aphrodite/plugin.yaml"
            ),
        },
    },
    "Pyright": {
        "reason": (
            "CI has no TTY, so `npx pyright@1.1.411` installs without prompting "
            "there; a local TTY needs `--yes` to avoid an interactive prompt"
        ),
        "replace": {
            "npx pyright@1.1.411 .": "npx --yes pyright@1.1.411 .",
        },
    },
}


# ---------------------------------------------------------------------------
# Minimal workflow parser
#
# PyYAML is not installed in this environment and the workflow uses a small,
# regular subset of YAML: jobs at indent 2, steps at indent 6, step keys at
# indent 8, nested with:/env: keys at indent 10, plus `run: |` block scalars.
# Parsing that directly keeps the checker dependency-free.
# ---------------------------------------------------------------------------


def indent_of(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def parse_step(lines: list[str], start: int) -> tuple[dict, int]:
    step: dict = {"name": None, "uses": None, "run": None, "with": {}, "env": {}}

    # Rewrite the opening "- name: ..." line as if it were an ordinary
    # 8-space-indented key, so one loop handles it.
    first = lines[start]
    block = [" " * 8 + first[8:]]
    i = start + 1
    while i < len(lines):
        line = lines[i]
        if not line.strip():
            block.append("")
            i += 1
            continue
        if indent_of(line) <= 6:
            break
        block.append(line)
        i += 1

    section = None
    j = 0
    while j < len(block):
        line = block[j]
        stripped = line.strip()
        indent = indent_of(line)
        if not stripped or stripped.startswith("#"):
            j += 1
            continue
        if indent == 8:
            key, _, value = stripped.partition(":")
            key = key.strip()
            value = value.strip()
            if key in ("with", "env") and not value:
                section = key
                j += 1
                continue
            section = None
            if key == "run" and value in ("|", ">", "|-", ">-", "|+", ">+"):
                body: list[str] = []
                j += 1
                while j < len(block) and (not block[j].strip() or indent_of(block[j]) > indent):
                    body.append(block[j])
                    j += 1
                while body and not body[-1].strip():
                    body.pop()
                if body:
                    cut = min(indent_of(item) for item in body if item.strip())
                    step["run"] = "\n".join(item[cut:].rstrip() for item in body).strip()
                else:
                    step["run"] = ""
                continue
            if key == "run":
                step["run"] = value
            elif key == "uses":
                step["uses"] = value.split(" #")[0].strip()
            elif key == "name":
                step["name"] = value.strip("'\"")
            j += 1
            continue
        if indent > 8 and section:
            key, _, value = stripped.partition(":")
            step[section][key.strip()] = value.strip().strip("'\"")
            j += 1
            continue
        j += 1

    return step, i


def parse_workflow(text: str) -> dict[str, list[dict]]:
    jobs: dict[str, list[dict]] = {}
    current_job = None
    in_jobs = False
    i = 0
    lines = text.splitlines()
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            i += 1
            continue
        indent = indent_of(line)
        if indent == 0:
            in_jobs = stripped == "jobs:"
            current_job = None
            i += 1
            continue
        if in_jobs and indent == 2 and stripped.endswith(":"):
            current_job = stripped[:-1]
            jobs.setdefault(current_job, [])
            i += 1
            continue
        if in_jobs and current_job and indent == 4 and stripped == "steps:":
            i += 1
            while i < len(lines):
                inner = lines[i]
                inner_stripped = inner.strip()
                if not inner_stripped or inner_stripped.startswith("#"):
                    i += 1
                    continue
                inner_indent = indent_of(inner)
                if inner_indent < 6:
                    break
                if inner_indent == 6 and inner_stripped.startswith("- "):
                    step, i = parse_step(lines, i)
                    jobs[current_job].append(step)
                    continue
                i += 1
            continue
        i += 1
    return jobs


def step_key(step: dict) -> tuple[str, str]:
    # An action step is identified by its action, not by its display name:
    # renaming `- name:` on a `uses:` step is cosmetic and must not read as a
    # new step. Run steps are identified by name, since they have no action.
    if step["uses"]:
        return ("uses", step["uses"].split("@")[0])
    if step["name"]:
        return ("name", step["name"])
    return ("unknown", "")


def step_label(step: dict) -> str:
    uses = (step["uses"] or "").split("@")[0]
    if step["name"] and uses:
        return f"{step['name']} [{uses}]"
    if step["name"]:
        return step["name"]
    if uses:
        return f"{uses} (action)"
    return "<unnamed step>"


# ---------------------------------------------------------------------------
# Command extraction
# ---------------------------------------------------------------------------

# Leading `VAR=value` assignment(s) on a command line.
ENV_ASSIGN_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)=(\S*)\s+")

# Lines that are shell plumbing, not the command under test.
SKIP_KEYWORDS = {
    "if",
    "then",
    "else",
    "elif",
    "fi",
    "for",
    "do",
    "done",
    "while",
    "until",
    "return",
    "exit",
    "local",
    "export",
    "set",
    "shift",
    "case",
    "esac",
    "true",
    "false",
}

FUNCTION_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)\(\)\s*\{$")

# A runner invocation of a mirrored alias: `RunStep "<label>" "<ci step>" cargo ci-x`.
# Anchored on RunStep so a comment or an echo that merely mentions an alias is
# not counted as running it.
RUNSTEP_RE = re.compile(r"^\s*RunStep\b.*?\bcargo\s+(ci-[a-z-]+)\s*$", re.MULTILINE)


def split_env(line: str) -> tuple[dict[str, str], str]:
    """Split leading `VAR=value` prefixes off a command line."""
    env: dict[str, str] = {}
    while True:
        match = ENV_ASSIGN_RE.match(line)
        if not match:
            return env, line
        env[match.group(1)] = match.group(2)
        line = line[match.end() :]


def command_lines(block: str) -> list[str]:
    """The command lines of a CI run: block, with comments and `cd` removed.

    `cd X && cmd` keeps only `cmd`: both sides change directory before running
    the same command, so the directory change is not a divergence - it is the
    runner's own `cd` line, which parse_functions() drops on the other side.
    """
    commands: list[str] = []
    for raw in block.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        while line.startswith("cd ") and "&&" in line:
            line = line.split("&&", 1)[1].strip()
        if line.startswith("cd ") or not line:
            continue
        commands.append(line)
    return commands


def parse_functions(script: str) -> dict[str, list[str]]:
    """Map each shell function name to the command lines it runs.

    Comments, blank lines, `cd` lines, `echo` lines and control-flow keywords are
    dropped; everything else is a command the function actually runs. That is the
    difference between "the function exists" and "the function runs the CI
    command": a changed body shows up here.
    """
    functions: dict[str, list[str]] = {}
    current: str | None = None
    for raw in script.splitlines():
        if current is None:
            match = FUNCTION_RE.match(raw)
            if match:
                name = match.group(1)
                current = name
                functions[name] = []
            continue
        if raw.strip() == "}":
            current = None
            continue
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("\\echo") or line.startswith("echo "):
            continue
        if line.startswith("cd "):
            continue
        if line.split()[0] in SKIP_KEYWORDS:
            continue
        functions[current].append(line)
    return functions


def expected_alias_value(step: dict) -> str | None:
    """The alias value that copies this CI step, or None if it is not a command."""
    uses = (step["uses"] or "").split("@")[0]
    if uses == DENY_ACTION:
        command = step["with"].get("command") or DENY_DEFAULT_COMMAND
        arguments = step["with"].get("arguments", DENY_DEFAULT_ARGUMENTS)
        parts = ["deny"]
        if arguments:
            parts.append(arguments)
        parts.append(command)
        return " ".join(parts)
    run = (step["run"] or "").strip()
    if not run:
        return None
    if run.startswith("cargo "):
        return run[len("cargo ") :]
    return None


def ci_commands(step: dict) -> tuple[list[str], dict[str, str]]:
    """The commands a CI step executes, and the env it sets for them."""
    uses = (step["uses"] or "").split("@")[0]
    if uses == RUFF_ACTION:
        args = step["with"].get("args") or RUFF_DEFAULT_ARGS
        src = step["with"].get("src") or RUFF_DEFAULT_SRC
        return [f"ruff {args} {src}"], {}
    return command_lines(step["run"] or ""), dict(step["env"])


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------


class Report:
    def __init__(self, verbose: bool) -> None:
        self.verbose = verbose
        self.failures: list[str] = []
        self.passed = 0

    def ok(self, message: str) -> None:
        self.passed += 1
        print(f"  PASS  {message}")

    def fail(self, message: str) -> None:
        self.failures.append(message)
        print(f"  FAIL  {message}")

    def note(self, message: str) -> None:
        if self.verbose:
            print(f"        {message}")


def parse_simple_toml(text: str) -> dict:
    """Fallback reader for the two small config files this checker reads.

    Handles only the shapes those files use: ``[table]`` headers,
    ``key = "string"`` and ``key = ["a", "b"]``. It exists because the macOS
    system python3 (3.9) has no ``tomllib``, and a checker that dies on an
    ImportError stops guarding the mirror while still exiting non-zero for the
    wrong reason. tomllib is used whenever it is importable.
    """
    document: dict = {}
    current: dict = document
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = document.setdefault(line[1:-1].strip(), {})
            continue
        key, separator, value = line.partition("=")
        if not separator:
            continue
        key = key.strip().strip("\"'")
        value = value.strip()
        if value.startswith("["):
            items = []
            for item in value.strip("[]").split(","):
                item = item.strip()
                if item:
                    items.append(item.strip("\"'"))
            current[key] = items
        elif value[:1] in ("'", '"'):
            quote = value[0]
            end = value.find(quote, 1)
            current[key] = value[1:end] if end > 0 else value[1:]
        else:
            current[key] = value
    return document


def load_toml(path: Path) -> dict:
    try:
        raw = path.read_bytes()
    except FileNotFoundError:
        raise SystemExit(f"error: {path} not found") from None

    if tomllib is None:
        return parse_simple_toml(raw.decode("utf-8"))

    try:
        return tomllib.loads(raw.decode("utf-8"))
    except tomllib.TOMLDecodeError as error:
        raise SystemExit(f"error: {path} is not valid TOML: {error}") from None


def check_aliases(report: Report, jobs: dict[str, list[dict]], aliases: dict[str, str]) -> None:
    used: dict[str, str] = {}
    for job, spec_steps in SPEC.items():
        actual = jobs.get(job, [])
        by_key: dict[tuple[str, str], dict] = {}
        for step in actual:
            by_key.setdefault(step_key(step), step)

        for key, where in spec_steps:
            if where[0] != "alias":
                continue
            alias = where[1]
            step = by_key.get(key)
            if step is None:
                continue  # already reported as a missing step
            used[alias] = job
            expected = expected_alias_value(step)
            if expected is None:
                report.fail(f"{job} / {step_label(step)}: no cargo command to mirror")
                continue
            actual_value = aliases.get(alias)
            if actual_value is None:
                report.fail(f"{alias} is missing from .cargo/config.toml")
                continue
            if actual_value != expected:
                report.fail(
                    f"{alias} does not copy {job} / {step_label(step)}\n"
                    f"          CI:    {expected}\n"
                    f"          alias: {actual_value}"
                )
                continue
            report.ok(f"{alias} == {job} / {step_label(step)}")
            report.note(f"CI:    {expected}")
            report.note(f"alias: {actual_value}")

    for alias in sorted(aliases):
        if alias.startswith("ci-") and alias not in used:
            report.fail(f"{alias} is defined in .cargo/config.toml but no CI step uses it")


def check_runner_order(report: Report, script: str) -> None:
    """The runner must invoke every alias, in CI's step order.

    Parsed from `RunStep ... cargo ci-<name>` lines, so a comment that mentions
    `cargo ci-check` - or a function nobody calls - does not satisfy coverage.
    """
    expected = [where[1] for job in SPEC for _, where in SPEC[job] if where[0] == "alias"]
    actual = RUNSTEP_RE.findall(script)

    if expected == actual:
        report.ok(f"Maintain/Check.sh runs {len(expected)} alias step(s) in CI order")
        return

    limit = max(len(expected), len(actual))
    for index in range(limit):
        want = expected[index] if index < len(expected) else "<no step>"
        have = actual[index] if index < len(actual) else "<no step>"
        if want == have:
            continue
        report.fail(
            f"Maintain/Check.sh alias step {index + 1} is `cargo {have}` but CI order "
            f"requires `cargo {want}` (only a RunStep invocation counts, not a comment)"
        )
        return


def apply_divergence(report: Report, function: str, commands: list[str]) -> tuple[list[str], bool]:
    """Apply the declared substitutions for one function. Returns (commands, fresh)."""
    entry = DIVERGENCE.get(function)
    if entry is None:
        return commands, True

    if not entry.get("reason"):
        report.fail(f"DIVERGENCE[{function!r}] declares no reason")
        return commands, False

    replace = entry["replace"]
    mapped: list[str] = []
    seen: set[str] = set()
    for command in commands:
        if command in replace:
            seen.add(command)
            mapped.append(replace[command])
        else:
            mapped.append(command)

    stale = sorted(set(replace) - seen)
    if stale:
        report.fail(
            f"{function}(): DIVERGENCE names a CI command Check.yml no longer runs:\n"
            f"          {stale[0]}\n"
            f"          Update DIVERGENCE and Maintain/Check.sh together."
        )
        return commands, False
    return mapped, True


def check_runner_reachability(
    report: Report, script: str, functions: dict[str, list[str]]
) -> None:
    """Every function that invokes a mirrored alias must be called by the router.

    Without this, moving ``RunStep ... cargo ci-check`` into a function nobody
    calls would still satisfy the ordered-invocation check.
    """
    called = set()
    for raw in script.splitlines():
        line = raw.strip()
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", line):
            called.add(line)

    for name, commands in sorted(functions.items()):
        if not any(RUNSTEP_RE.match(command) for command in commands):
            continue
        if name not in called:
            report.fail(f"Maintain/Check.sh defines {name}() with alias steps but never calls it")
            continue
        report.ok(f"{name}() is called from the router")


def check_scripts(
    report: Report, jobs: dict[str, list[dict]], functions: dict[str, list[str]]
) -> None:
    for job, spec_steps in SPEC.items():
        by_key = {step_key(step): step for step in jobs.get(job, [])}
        for key, where in spec_steps:
            if where[0] != "script":
                continue
            function = where[1]
            step = by_key.get(key)
            if step is None:
                continue
            body = functions.get(function)
            if body is None:
                report.fail(
                    f"{job} / {step_label(step)}: Maintain/Check.sh defines no {function}() step"
                )
                continue

            expected, expected_env = ci_commands(step)
            expected, fresh = apply_divergence(report, function, expected)
            if not fresh:
                continue

            actual: list[str] = []
            actual_env: dict[str, str] = {}
            for line in body:
                env, command = split_env(line)
                actual_env.update(env)
                actual.append(command)

            if expected != actual:
                report.fail(
                    f"{job} / {step_label(step)}: {function}() does not run the CI command(s)\n"
                    f"          CI:    {' | '.join(expected) or '<none>'}\n"
                    f"          local: {' | '.join(actual) or '<none>'}"
                )
                continue
            if expected_env != actual_env:
                report.fail(
                    f"{job} / {step_label(step)}: {function}() sets different env\n"
                    f"          CI:    {expected_env}\n"
                    f"          local: {actual_env}"
                )
                continue
            report.ok(f"{function}() runs {job} / {step_label(step)}")
            report.note(f"command: {' | '.join(actual)}")


def check_divergence_inventory(report: Report) -> None:
    declared = {where[1] for job in SPEC for _, where in SPEC[job] if where[0] == "script"}
    for name in sorted(DIVERGENCE):
        if name not in declared:
            report.fail(f"DIVERGENCE names {name}() but no CI step is mirrored by it")
            continue
        report.ok(f"DIVERGENCE[{name!r}] is declared with a reason")


def check_toolchain(report: Report, jobs: dict[str, list[dict]], toolchain: dict) -> None:
    pinned = toolchain.get("toolchain", {})
    channel = pinned.get("channel")
    components = sorted(pinned.get("components", []))
    if not channel:
        report.fail("rust-toolchain.toml has no [toolchain] channel")
        return

    found = 0
    for job, steps in jobs.items():
        for step in steps:
            if (step["uses"] or "").split("@")[0] != TOOLCHAIN_ACTION:
                continue
            found += 1
            ci_channel = step["with"].get("toolchain")
            ci_components = sorted(
                item.strip()
                for item in step["with"].get("components", "").split(",")
                if item.strip()
            )
            if ci_channel != channel:
                report.fail(
                    f"{job} / {step_label(step)}: toolchain {ci_channel!r} != "
                    f"rust-toolchain.toml channel {channel!r}"
                )
                continue
            if ci_components != components:
                report.fail(
                    f"{job} / {step_label(step)}: components {ci_components} != "
                    f"rust-toolchain.toml components {components}"
                )
                continue
            report.ok(f"{job} / {step_label(step)}: {channel} with {', '.join(components)}")
    if found == 0:
        report.fail(f"no {TOOLCHAIN_ACTION} step found; the toolchain pin is unenforced")


def check_inventory(report: Report, jobs: dict[str, list[dict]]) -> None:
    expected_jobs = set(SPEC)
    actual_jobs = set(jobs)
    for job in sorted(expected_jobs - actual_jobs):
        report.fail(f"job {job!r} is declared here but missing from Check.yml")
    for job in sorted(actual_jobs - expected_jobs):
        report.fail(f"job {job!r} exists in Check.yml but is not mirrored (add it to SPEC)")

    for job in sorted(expected_jobs & actual_jobs):
        expected = [key for key, _ in SPEC[job]]
        actual = [step_key(step) for step in jobs[job]]
        if expected == actual:
            report.ok(f"{job}: {len(actual)} step(s) in CI order")
            continue
        limit = max(len(expected), len(actual))
        for index in range(limit):
            want = expected[index] if index < len(expected) else None
            have = actual[index] if index < len(actual) else None
            if want == have:
                continue
            have_label = step_label(jobs[job][index]) if have else "<no step>"
            want_label = want[1] if want else "<no step>"
            report.fail(
                f"{job}: step {index + 1} is {have_label!r} in Check.yml "
                f"but {want_label!r} in the mirror table"
            )
            break
        else:
            report.ok(f"{job}: step set matches")


def main() -> int:
    parser = argparse.ArgumentParser(description="Check.yml <-> local mirror parity")
    parser.add_argument("--workflow", type=Path, default=DEFAULT_WORKFLOW)
    parser.add_argument("--cargo-config", type=Path, default=DEFAULT_CARGO_CONFIG)
    parser.add_argument("--toolchain", type=Path, default=DEFAULT_TOOLCHAIN)
    parser.add_argument("--script", type=Path, default=DEFAULT_SCRIPT)
    parser.add_argument("--verbose", action="store_true", help="print every command pair")
    args = parser.parse_args()

    for path in (args.workflow, args.cargo_config, args.toolchain, args.script):
        if not path.is_file():
            print(f"error: {path} not found", file=sys.stderr)
            return 2

    jobs = parse_workflow(args.workflow.read_text(encoding="utf-8"))
    if not jobs:
        print(f"error: no jobs parsed from {args.workflow}", file=sys.stderr)
        return 2

    aliases = load_toml(args.cargo_config).get("alias", {})
    toolchain = load_toml(args.toolchain)
    script = args.script.read_text(encoding="utf-8")
    functions = parse_functions(script)

    print("Check.yml <-> local mirror parity")
    print("")
    print(f"  workflow: {args.workflow}")
    print(f"  aliases:  {args.cargo_config}")
    print(f"  toolchain:{args.toolchain}")
    print(f"  runner:   {args.script}")
    print("")

    report = Report(args.verbose)
    check_inventory(report, jobs)
    check_aliases(report, jobs, aliases)
    check_runner_order(report, script)
    check_runner_reachability(report, script, functions)
    check_scripts(report, jobs, functions)
    check_divergence_inventory(report)
    check_toolchain(report, jobs, toolchain)

    print("")
    if report.failures:
        print(f"FAIL: {len(report.failures)} divergence(s), {report.passed} check(s) passed.")
        print("")
        print("The local gates no longer match Check.yml. Either the workflow moved")
        print("(update SPEC, .cargo/config.toml and Maintain/Check.sh together) or the")
        print("mirror drifted (restore the CI command verbatim).")
        return 1

    print(
        f"OK: {report.passed} check(s) passed - command/config inventory consistent with Check.yml."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
