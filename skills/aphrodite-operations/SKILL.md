---
name: aphrodite-operations
description: Use when operating inside an aphrodite-compressed session or the
    Aphrodite repo. Engine compression workflow, dual-mode rebuild, standalone
    repo sync, dep pinning, breakpoint pitfalls.
version: 1.1.0
platforms: [macos]
tags: [aphrodite, ccr, compression, operations, rebuild]
---

# Aphrodite Operations

Day-to-day operational patterns for working inside an aphrodite-compressed
session and the Aphrodite repo.

## When to Use

- Tool output arrives as `<<<CCR:hash|type|size>>>` markers and you must work
  inside the compression engine instead of against it.
- You need to rebuild the binary, sync the standalone plugin repo, or verify
  plugin breakpoints after changes.

## Compressed-Session Workflow

The engine compresses every read — never fight it by re-reading the same file
with different offsets or tools. The full tool-API doctrine lives in
`aphrodite-tool-testing`; the operational shape is:

1. **Plan reads ahead** — `aphrodite_prefetch(paths=[...])` reads and
   compresses files in the background; track progress with
   `aphrodite_prefetch_status`.
2. **Retrieve, don't re-read** — on `<<<CCR:hash|type|size>>>`, call
   `aphrodite_retrieve(hash)`. Never call `read_file` again on the same file.
3. **Write terminal output to files** — `cmd > /tmp/out.txt 2>&1`, then
   prefetch/retrieve the file instead of reading raw output.
4. **Do other work while waiting** — dispatch prefetches and independent tasks,
   then poll readiness.

Anti-pattern: calling `read_file` 3+ times on the same file with different
offsets — each call returns a fresh compressed marker.

## Dual-Mode Rebuild (`aphrodite_rebuild`)

- **Dev mode**: a parent `Cargo.toml` exists → builds from Rust source via
  `cargo build --release`.
- **User mode**: no Cargo workspace found (standalone install) → re-downloads
  the binary from GitHub Releases.

Never hardcode `os.path.dirname()` chains in `_hooks/rebuild.py` — they break
when directory structure changes. Always use `_find_cargo_toml()`, which walks
up to 6 levels from the rebuild module:

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

## `--version` Must Precede Config Loading

The Rust binary's `--version` is only parsed by clap when `Cli::parse()` runs —
which never happens when `aphrodite.toml` exists. Intercept `--version`/`-V` at
the top of `main()` before config loading, or `_check_binary_version()` (which
calls `[BINARY, "--version"]`) hangs:

```rust
let args: Vec<String> = std::env::args().collect();
if args.iter().any(|a| a == "--version" || a == "-V") {
    println!("aphrodite v{}", env!("CARGO_PKG_VERSION"));
    return Ok(());
}
```

## Standalone Plugin Repo Sync

End users install from
[Aphrodite-Hermes](https://github.com/PlayForm/Aphrodite-Hermes) — a lightweight
repo with Python plugin files only (no binary, no monorepo). The binary is
auto-downloaded from
[GitHub Releases](https://github.com/PlayForm/Aphrodite/releases) on first
session start. After releasing in the monorepo, sync the changed files:

```bash
cp plugins/aphrodite/_core/config.py $STANDALONE/_core/config.py
cp plugins/aphrodite/plugin.yaml $STANDALONE/plugin.yaml
cp plugins/aphrodite/pyproject.toml $STANDALONE/pyproject.toml
cp plugins/aphrodite/__init__.py $STANDALONE/__init__.py
cp plugins/aphrodite/_hooks/rebuild.py $STANDALONE/_hooks/rebuild.py
cd $STANDALONE && git add -A && git commit -m "sync: v$NEW" && git push
```

## Dep Pinning Convention

Pin all dependencies to exact versions — never semver ranges:

| Lang   | Wrong            | Right                |
| ------ | ---------------- | -------------------- |
| Python | `ruff>=0.13`     | `ruff==0.15.17`      |
| Cargo  | `tower = "0.5"`  | `tower = "0.5.3"`    |
| Cargo  | `anyhow = "1"`   | `anyhow = "1.0.102"` |

Check the latest version before bumping: `pip3 index versions <pkg>` or
`cargo search <pkg> --limit 1`. Let Dependabot drive future updates.

## Breakpoint Pitfalls (silent breakage)

- tracing `DisplayValue<T>` requires `fmt::Display` — never format a `PathBuf`
  with `%`; use `.display()`.
- Never import `_headroom_context` from `.env` — it lives in `.health`; the
  wrong import silently kills the plugin at load.
- axum 0.8 route wildcards: `/*path` is invalid at startup — use `/{*path}`.
  See `aphrodite-cargo-upgrade` for the full cargo-upgrade breakpoint list.
- Never edit plugin-shipped skills through `skill_manage` — they are read-only;
  write the file directly or create a profile-level skill for operational
  patterns.