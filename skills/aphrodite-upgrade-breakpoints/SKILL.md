---
name: aphrodite-upgrade-breakpoints
description: Cargo upgrade breakpoints for aphrodite + headroom — required by the standalone plugin repo and release workflow.
version: 1.0.1
---

# Aphrodite Upgrade Breakpoints

Checklist of things that break silently when upgrading cargo deps or refactoring.

## Rebuild Path Resolution

`_hooks/rebuild.py` lives at `plugins/aphrodite/_hooks/rebuild.py`. Counting `os.path.dirname()` calls is fragile. Use `_find_cargo_toml()` which walks up the tree:

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

## --version Flag

Rust binary `--version` only works if handled BEFORE config loading. When `aphrodite.toml` exists, clap's `Cli::parse()` (with `#[command(version)]`) is never called. Add early check in `main()`:

```rust
let args: Vec<String> = std::env::args().collect();
if args.iter().any(|a| a == "--version" || a == "-V") {
    println!("aphrodite v{}", env!("CARGO_PKG_VERSION"));
    return Ok(());
}
```

## Standalone Plugin Repo

End users install from `PlayForm/Aphrodite-Hermes` — NO binary committed (74 files, 317KB). Binary auto-downloaded from GitHub Releases on first `on_start()` via `_ensure_binary()`. The `_rebuild_handler` checks for Cargo.toml; if absent it re-downloads from releases.

## Previous Breakpoints

- axum 0.8 wildcards in route patterns
- tracing `DisplayValue<T>` requiring `fmt::Display` — `PathBuf` uses `.display()` not `%` format
- `_headroom_context` import from `.health` NOT `.env` (silent plugin kill)
