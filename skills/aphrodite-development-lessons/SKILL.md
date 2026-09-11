---
name: aphrodite-development-lessons
description: "Use when developing the aphrodite plugin. Session setup, symlink, env_passthrough, dual-store, version bump locations, release notes pitfalls."
version: 1.2.0
platforms: [macos]
tags: [aphrodite, development, pitfalls]
---

# Aphrodite Development Lessons

Imperative pitfalls for developing the aphrodite plugin (monorepo
`PlayForm/Aphrodite` + standalone `PlayForm/Aphrodite-Hermes`). Each lesson is
a rule plus the failure it prevents.

## Enable auto-expand while developing

When auto-expand is off, every file read gets compressed into `<<<CCR:hash>>>`
markers - the LLM sees markers instead of content and debugging becomes a
cascading failure loop. For development sessions, enable it:

```toml
[compression]
auto_expand = true
auto_expand_limit = 51200
```

or per-session: `APHRODITE_AUTO_EXPAND=1 hermes --profile dev-aphrodite`

For the full mechanism and test protocol, see `aphrodite-auto-expand-testing`.

**When you see a CCR marker**, retrieve it with `aphrodite_retrieve(hash)` -
never try alternative tools; they produce CCR markers too.

## Symlink the plugin to the repo

The plugin must symlink directly to the repo for instant code updates:

```
~/.hermes/profiles/dev-aphrodite/plugins/aphrodite → /path/to/Aphrodite/plugins/aphrodite
```

Without the symlink, code changes in the repo never reach the running plugin
and background workers keep running stale code.

## env_passthrough - API Key Blocking

Empty `env_passthrough: []` blocks `APHRODITE_API_KEY` from reaching proxy
subprocesses - proxies silently fail to start. Set it explicitly:

`hermes config set terminal.env_passthrough '["APHRODITE_API_KEY","PATH","HOME"]' --profile dev-aphrodite`

## Dual-Store Guarantee (CCR_UNRESOLVED Fix)

Every proxy compress/fetch MUST also store in the inline store - this prevents
`[CCR_UNRESOLVED]` from race conditions or proxy eviction:

- After `_compress_via_proxy(content, ...)` succeeds with hash `h`, call
  `_inline_store_put(h, content)` before returning the marker.
- After `_resolve_one` fetches from proxy, call
  `_inline_store_put(hash_val, result)` before returning.

## Version Bump Locations

The 5-location set: `Cargo.toml` (binary), `_core/config.py`
(`BIN_VERSION` + `PLUGIN_VERSION`), `pyproject.toml`, `__init__.py` docstring,
`plugin.yaml`. Use `--minor` for feature releases, default patch for fixes. The
full binary+plugin version-sync list lives in `aphrodite-release-workflow`.

## Release Notes

Never pass inline backtick-quoted text to `gh release create --notes` - the
shell interprets backticks as command substitution; always use `--notes-file`
with a heredoc. Never ship a bare compare link as release notes; every release
MUST include Summary, Changes, Infrastructure, What Ships, and Links. Template:
`.hermes/RELEASE-TEMPLATE.md`; full content standards live in
`aphrodite-release-workflow`.

## Session Setup Checklist

Before development:

1. Auto-expand ON: `aphrodite_catalog(mode='toc')` shows inline content
2. Proxies running: `aphrodite_stats` shows `token: on`, `cache: on`
3. Plugin symlinked to repo
4. env_passthrough set
5. Context engine ON: `[compression].context_engine = true` in `aphrodite.toml`

## Pitfalls

- never rely on plugin-shipped skills being editable - they are read-only via
  skill_manage; keep operational patterns in profile-level skills
- never assume a new import is safe - importing a symbol that the target module
  lacks silently kills the plugin at session start; after adding imports, test
  `python3 -c "import aphrodite"`