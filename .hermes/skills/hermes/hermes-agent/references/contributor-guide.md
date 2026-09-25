# Contributor Quick Reference - Aphrodite Monorepo

Use when contributing to PlayForm/Aphrodite (the CCR compression proxy plugin
for Hermes) on the public **Development** branch: building, testing, adding a
CCR tool, or following repo conventions. Full developer docs for the Hermes
host the plugin runs inside: https://hermes-agent.nousresearch.com/docs/developer-guide/

## Project Layout

```
PlayForm/Aphrodite/
├── crates/aphrodite/         # Rust crate - the CCR compression proxy engine
├── crates/aphrodite-hermes/  # Rust crate - Hermes bridge; builds the cdylib
│                             #   libaphrodite_hermes.dylib (hot-reloaded)
├── plugins/aphrodite/        # Plugin submodule (remote 'Source':
│                             #   PlayForm/Aphrodite-Hermes)
├── vendor/                   # Vendored deps: vendor/headroom, vendor/rtk
├── Build.yml                 # CI pipeline
├── Publish.yml               # Release pipeline
├── .hermes/AGENTS.md         # Repo facts, quality gates
├── .hermes/skills/           # Dev skills (Development branch only, never shipped)
└── .hermes/tmp/              # Scratch (gitignored contents, tracked .gitkeep)
```

Branches: **Development** (public integration line) and **Current** (releases).
Runtime home: `~/.hermes/aphrodite/` (`aphrodite.toml`, `binaries/`,
`hotreload/`). Dev profile: `dev-aphrodite` (`hermes --profile
dev-aphrodite`; plugin symlinked at
`~/.hermes/profiles/dev-aphrodite/plugins/aphrodite`).

## Dev loop

```sh
# Pane 0 - watch BOTH packages, or the dylib never rebuilds
cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x 'run -p aphrodite'
# Pane 1 - test in production
hermes --profile dev-aphrodite
```

Verify with `aphrodite_rebuild` (dylib version + proxy health) and
`aphrodite_stats` (session/proxy counters) before trusting a change.

## Adding a CCR tool

The CCR tool family (`aphrodite_retrieve`, `aphrodite_compress`,
`aphrodite_stats`, `aphrodite_rebuild`, `aphrodite_search`,
`aphrodite_catalog`, `aphrodite_diff`, `aphrodite_directive`,
`aphrodite_files`, `aphrodite_prefetch`, `aphrodite_prefetch_status`,
`aphrodite_reclassify`, `aphrodite_test`) is exposed to the agent through the
plugin. Two things make a tool usable:

1. **Register it** in the plugin's tool layer (the loader file set in
   `plugins/aphrodite/`: `plugin.yaml`, `BINARY_VERSION`, `_bindings.py`).
2. **Expose it in a toolset** - a registered tool is only visible once its
   name appears in a toolset.

All handlers return JSON strings. Use `get_hermes_home()` for paths, never
hardcode `~/.hermes`. Any new tool that fetches compressed content must be
added to the transform skip list so its own output is never re-compressed
into a marker (see `aphrodite-compression-safety`).

## Plugin hooks into the agent loop

The plugin attaches to the Hermes conversation loop via these hooks -
implemented in the Rust bridge (`crates/aphrodite-hermes`) or the Python
binding layer:

- `on_session_start` - fire-and-forget session bootstrap
- `transform_tool_result` - replaces a tool result (this is where CCR markers
  are produced)
- `pre_llm_call` - injects context before the LLM call
- `transform_terminal_output` - replaces terminal output
- `post_llm_call` - fires after the LLM responds

**Invariant for hook authors**: `conversation_history` arrives as a COPY -
in-place mutations are discarded. Never mutate it to "fix" history.

## Testing

```sh
cargo test -p aphrodite -p aphrodite-hermes # Rust suites
```

- Round-trip smoke test: `aphrodite_test` (compress + retrieve).
- Probe in production: `hermes --profile dev-aphrodite`, then
  `aphrodite_rebuild` / `aphrodite_stats`.
- Scratch belongs in `.hermes/tmp/` (gzip bulky fixtures in place), never a
  system temp dir.
- Dev skills in `.hermes/skills/` are edited directly with `write_file`/
  `patch`, never via `skill_manage`.

**Cross-platform guards (Python-side tests):** POSIX-only syscalls need a
skip marker on win32 - symlink creation, POSIX file modes (0o600), and
`signal.SIGALRM` are the common ones. Monkeypatching `sys.platform` is not
enough when the code under test also calls `platform.system()` - patch all
three together. The Aphrodite plugin itself targets macOS; the guards protect
the shared Hermes test suite.

## System prompt's execution-environment block

Factual host/backend guidance (OS, `$HOME`, cwd, terminal backend, shell) is
emitted by the host's `build_environment_hints()`. The invariant for prompt
authors: with a **remote** terminal backend, host info is suppressed and
every file tool runs inside the backend container - the prompt must never
describe a host the agent can't touch.

## Commit Conventions

```
type: concise subject line

Optional body.
```

Types: `fix:`, `feat:`, `refactor:`, `docs:`, `chore:`

## Key Rules

- **Never break prompt caching** - don't change context, tools, or system
  prompt mid-conversation.
- **Message role alternation** - never two assistant or two user messages in
  a row.
- Use `get_hermes_home()` from `hermes_constants` for all paths
  (profile-safe).
- Config values go in `config.yaml`, secrets go in `.env` - never in
  `~/.hermes/aphrodite/aphrodite.toml` or plugin source.
- A `<<<CCR:hash|type|size>>>` marker in tool output is stored content -
  resolve it with `aphrodite_retrieve(hash)` before acting; never re-read the
  source file.
- Version bumps and release notes are owned by the release skills
  (`aphrodite-release-flow` / `aphrodite-release-workflow`) - never bump or
  announce versions ad hoc.

**Stop if** a session is not in the `Development` branch or the plugin
symlink does not resolve to `$PWD/plugins/aphrodite` - stale binaries
silently test old code.

**Recovery** - permitted: run the orientation gate (5 read-only commands),
fix the symlink, restart Pane 1. Prohibited: switching branches or cleaning
the working tree to "fix" the gate.

## Verification

| Claim                          | Test                                                          | Pass condition                      |
| ------------------------------ | ------------------------------------------------------------- | ----------------------------------- |
| Workspace builds               | `cargo build -p aphrodite -p aphrodite-hermes`                | Exit 0; dylib produced              |
| Round trip works               | `aphrodite_test`                                              | Compress + retrieve returns payload |
| Plugin resolves to repo source | `readlink ~/.hermes/profiles/dev-aphrodite/plugins/aphrodite` | Symlink == `$PWD/plugins/aphrodite` |
| Scratch stays hermetic         | `git status --short` after a probe                            | No artifacts outside `.hermes/tmp/` |
