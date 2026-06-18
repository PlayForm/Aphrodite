---
name: aphrodite-v0.8.6-patterns
description:
    Development patterns learned across v0.8.5→v0.8.6 - Cargo.toml walk-up,
    --version early handling, dep pinning, standalone plugin repo,
    prefetch-first workflow.
version: 1.0.0
platforms: [macos]
related_skills:
    [
        aphrodite-dev-workflow,
        aphrodite-development-lessons,
        aphrodite-context-efficiency,
    ]
---

# Aphrodite v0.8.6 Patterns

Techniques and pitfalls from the v0.8.6 development cycle.

## Repo Path Resolution - `_find_cargo_toml()`

Brittle `os.path.dirname()` × N counting breaks when files move.
`_hooks/rebuild.py` was 3 levels from `plugins/` but 4 from repo root.

**Fix:** walk-up search:

```python
def _find_cargo_toml():
    d = os.path.dirname(os.path.abspath(__file__))
    for _ in range(6):
        if os.path.isfile(os.path.join(d, "Cargo.toml")):
            return d
        parent = os.path.dirname(d)
        if parent == d: break
        d = parent
    return None
```

Use in `_rebuild_handler`, `_binary.py` local-build fallback, and anywhere that
needs the Rust workspace root.

## `--version` Flag - Early Handling

Rust binary's `--version` only fires through `Cli::parse()`, skipped when
`aphrodite.toml` exists (multi-proxy path). `_check_binary_version()` hangs.

**Fix:** add to top of `main()`:

```rust
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("aphrodite v{}", option_env!("APHRODITE_VERSION")
            .unwrap_or(env!("CARGO_PKG_VERSION")));
        return Ok(());
    }
    // ... worker threads + run()
}
```

## Dep Pinning Convention

All deps pinned to exact versions - no semver ranges:

| Lang   | Wrong           | Right                |
| ------ | --------------- | -------------------- |
| Python | `ruff>=0.13`    | `ruff==0.15.17`      |
| Cargo  | `tower = "0.5"` | `tower = "0.5.3"`    |
| Cargo  | `anyhow = "1"`  | `anyhow = "1.0.102"` |

Check latest before bumping: `pip3 index versions <pkg>` /
`cargo search <pkg> --limit 1`. Dependabot handles future updates.

## Standalone Plugin Repo

End users must NOT clone the `PlayForm/Aphrodite` monorepo. Created
`PlayForm/Aphrodite-Hermes`:

- **74 files, 317KB** - Python source, plugin.yaml, skills, no binary
- Binary auto-downloaded via `_ensure_binary()` from GitHub Releases
- `_rebuild_handler`: checks `Cargo.toml` → builds from source (dev) or
  downloads (user)
- Install:
  `hermes plugin install git:https://github.com/PlayForm/Aphrodite-Hermes.git`
- README points to main repo with absolute links, same PlayForm branding
  (`# [Name] 💋 `)

## Prefetch-First Workflow

When tool output is compressed, use async tools instead of fighting:

- `aphrodite_prefetch()` for files you'll need - reads in background
- `terminal(background=true, notify_on_complete=true)` for builds/tests
- `process(action='poll')` to check progress, never `process(action='wait')`
- Pre-plan calls - dispatch, work, retrieve when needed
