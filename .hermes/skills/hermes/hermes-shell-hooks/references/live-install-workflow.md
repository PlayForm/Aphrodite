# Aphrodite Live-Edit Development Workflow (Plugin + Hot-Reload)

When contributing changes to Aphrodite (especially CCR plugin hooks or the
compression proxy), work directly on the repo source: the plugin submodule at
`plugins/aphrodite` (remote `Source` → `PlayForm/Aphrodite-Hermes`) and the
crates `crates/aphrodite` + `crates/aphrodite-hermes`. The dev profile
`dev-aphrodite` symlinks the plugin, and the dylib hot-reloads from
`~/.hermes/aphrodite/hotreload/` - edits take effect on mtime without a
separate install step.

## Setup

```bash
cd <workspace>   # Aphrodite repo root

# Orientation gate first (aphrodite-orientation): root, branch, submodule, remote
git rev-parse --show-toplevel
git branch --show-current        # expect: Development
git submodule status --recursive # plugins/aphrodite must resolve

# Prove source mode and the plugin binding
test -f Cargo.toml && test -f crates/aphrodite/Cargo.toml && test -d crates/aphrodite-hermes
echo "WORKSPACE:$?"
readlink ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite   # must resolve to $PWD/plugins/aphrodite
```

## Making Changes

Make surgical edits to the live source. The plugin is hot-reloaded, so changes
take effect for the next agent session (or immediately on dylib mtime).

Key files for hook/CCR system changes:

- `crates/aphrodite-hermes/` - the bridge crate that builds
  `libaphrodite_hermes.dylib` (CCR plugin hooks: `on_session_start`,
  `transform_tool_result`, `pre_llm_call`, `transform_terminal_output`,
  `post_llm_call`)
- `crates/aphrodite/` - the CCR compression proxy engine
- `plugins/aphrodite/` - plugin Python (loader files, tool registrations)
- `~/.hermes/aphrodite/aphrodite.toml` - runtime config (never edited by the
  agent's tool loop directly; use the owning skill's procedure)

Dev loop:

```bash
# Pane 0 - watch BOTH packages (single -p aphrodite never rebuilds the dylib)
cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x 'run -p aphrodite'

# Pane 1 - test in production
hermes --profile dev-aphrodite
```

## Testing

**Critical rule: test hooks only under ~/.hermes/ - never inside the repo.**
Post_tool_call hooks (like normalize-dashes.sh) modify files in-place by
re-reading them from disk after the write. Testing inside the Aphrodite repo
working tree will touch files you didn't intend to edit and create
mtime-change warnings. Use `~/.hermes/tmp/test-hook.txt` or similar scratch
paths (never `/tmp`).

CLI-mode oneshots are the correct test vehicle - they fire the full hook
pipeline through the agent's tool loop:

```bash
HERMES_ACCEPT_HOOKS=1 hermes --profile dev-aphrodite -z \
	"Write exactly: hello - world (with em dash) to ~/.hermes/tmp/test-hook.txt"
```

After a crate edit, verify the running binary is the one just built:

```bash
aphrodite_rebuild # reported dylib version must match the fresh build
aphrodite_stats   # proxy health counters
```

Python test suite commands (run from the repo root):

```bash
# Run the plugin test suite
python -m pytest plugins/aphrodite/tests -v --tb=short -n 0

# Quick import check after adding plugin imports (a missing symbol kills the
# plugin at session start)
python3 -c "import aphrodite"
```

**Important:** tests that auto-redirect `HERMES_HOME` to temp dirs never touch
the real `~/.hermes/` config, sessions, or hooks.

## Committing & Pushing

Use the repo commit conventions:

```
type: concise subject line

Optional body.
```

Types: `fix:`, `feat:`, `refactor:`, `docs:`, `chore:`

- Plugin source changes belong in the submodule repo
  (`PlayForm/Aphrodite-Hermes`, remote `Source`), parent repo changes in
  `PlayForm/Aphrodite`.
- Target branch for both: `Development`.

## Opening a PR

After pushing, GitHub prints a PR link. Target: `Development` branch of
`PlayForm/Aphrodite` (or `PlayForm/Aphrodite-Hermes` for plugin-submodule
changes).

## Key Design Principle: Single Hook Invocation

When adding response types to `pre_tool_call` hooks, the hook must fire
**exactly once** per tool execution. The `_dispatch_pre_tool_call_hooks()`
function was created specifically to solve this - it calls `invoke_hook()`
once and processes both block and modify responses from the same results
list. Never call `invoke_hook("pre_tool_call", ...)` from two different
places in the same tool dispatch path - shell hook callbacks have side
effects and double-firing corrupts behavior.

## Wire Protocol Conventions

Hermes supports two wire formats for hook responses to ease porting from
Claude Code:

| Concept | Hermes-canonical                        | Claude-Code-style                             |
| ------- | --------------------------------------- | --------------------------------------------- |
| Block   | `{"action": "block", "message": "..."}` | `{"decision": "block", "reason": "..."}`      |
| Modify  | `{"action": "modify", "args": {...}}`   | `{"decision": "modify", "tool_input": {...}}` |
