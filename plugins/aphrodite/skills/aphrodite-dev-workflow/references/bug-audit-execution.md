# Systematic Bug Audit Execution

## How to execute a batch of bug fixes from numbered task files

### 1. Load all task files first

```bash
# .hermes/tasks/ contains numbered audit files
ls .hermes/tasks/
# 1-wave-audit-v050-v0550.md
# 2-python-plugin-bugs-48-58.md
# 3-proxy-rs-bugs-59-70.md
# 4-main-retrieve-config-bugs-71-91.md
```

Read ALL of them before making any changes. They cross-reference each other.

### 2. Build a priority-ordered TODO

Sort bugs by severity:
- 🔴 Critical (fix NOW — crashes, security, data loss)
- 🟠 High (fix next — wrong behavior, silent failures)
- 🟡 Medium (fix after — performance, correctness edge cases)
- 🟢 Low/Improvement (nice-to-have)

### 3. Batch edits by file

Fix all bugs in ONE file before moving to the next. Don't jump between proxy.rs and __init__.py in the same batch.

### 4. Edit directly, don't test between fixes

When told "stop using terminal, just fix and develop":
- Make ALL code changes via `patch`/`write_file`
- Do NOT run `cargo build` between individual fixes
- Batch all edits, then test ONCE at the end
- Hitting the tool-call guardrail from repeated test failures means you should have been editing, not testing

### 5. After all edits: build + test + bump + commit + push

```bash
cargo build --release -p aphrodite
cp target/release/aphrodite ~/.hermes/aphrodite/aphrodite
cargo test -p aphrodite
```

Then bump versions in ALL 4 locations:
- `crates/aphrodite/Cargo.toml` (version)
- `plugins/aphrodite/__init__.py` (BIN_VERSION + PLUGIN_VERSION)
- `plugins/aphrodite/plugin.yaml` (version)

Then commit, tag, release.

### 6. Organize .hermes/ after each wave

- Create/update `MASTER-TASKS.md` — comprehensive table with status/severity/version
- Rename task files with descriptive prefixes
- Remove duplicates
- Update `HANDOFF.md` and `AGENTS.md`
- Commit with `git add --force .hermes/`

### 7. Sync plugin to all profiles

After plugin changes, copy to all 6 test profiles:
```bash
for prof in aphrodite-proxy-cache aphrodite-proxy-token ... ; do
  cp plugins/aphrodite/*.py plugins/aphrodite/plugin.yaml \
    ~/.hermes/profiles/$prof/plugins/aphrodite/
done
```

### Common Pitfalls

- **Python `F821 undefined-name`**: Missing imports after splitting modules. Fix: add `from ._core import ...` to the module that references the symbol.
- **Circular imports**: `_hooks` imports `_engine`, `_engine` imports `_hooks`. Fix: move shared symbols to `_core.py`.
- **`F811 redefined-while-unused`**: Duplicate definitions after state moved to `_core`. Fix: remove local definitions.
- **`default_value_t` with `PathBuf`**: Clap's `default_value_t` requires `Display`. Use `default_value = ""` and resolve at build_state time with XDG fallback.
- **Escape-drift in patch**: f-strings with escaped quotes cause spurious backslashes. Re-read file and use verbatim content.
