---
name: aphrodite-development-lessons
description: "Critical development pitfalls for Aphrodite — auto-expand, env_passthrough, plugin symlink, dual-store pattern, release notes, and session setup checklist. Learned across v0.5.104–v0.7.0."
version: 1.0.0
platforms: [macos]
related_skills: [aphrodite-dev-workflow, aphrodite-release-workflow, aphrodite-tool-guide]
---

# Aphrodite Development Lessons

Critical pitfalls and fixes learned across 100+ releases.

## Auto-Expand: The Critical Switch

When auto-expand is OFF, every file read gets compressed into `<<<CCR:hash>>>` markers. The LLM sees markers instead of content, creating a cascading failure loop where no debugging is possible.

**Fix:** enable auto-expand in `aphrodite.toml`:
```toml
[compression]
auto_expand = true
auto_expand_limit = 51200
```

Or per-session: `APHRODITE_AUTO_EXPAND=1 hermes --profile dev-aphrodite`

**When you see a CCR marker:** use `aphrodite_retrieve(hash)` — DO NOT try alternative tools. They'll also produce CCR markers.

## Plugin Symlink → Repo

The plugin must symlink directly to the repo for instant code updates:
```
~/.hermes/profiles/dev-aphrodite/plugins/aphrodite → /path/to/HermesCompress/plugins/aphrodite
```

Without this, code changes in the repo don't reach the running plugin. Background workers get stale code.

## env_passthrough — API Key Blocking

Empty `env_passthrough: []` blocks `APHRODITE_API_KEY` from reaching proxy subprocesses. Proxies silently fail to start.

**Fix:** `hermes config set terminal.env_passthrough '["APHRODITE_API_KEY","PATH","HOME"]' --profile dev-aphrodite`

## Release Notes — Shell Injection

**NEVER** use inline backtick-quoted text with `gh release create --notes`. The shell interprets backticks as command substitution, capturing Hermes TUI output.

**Fix:** use `--notes-file` with a heredoc:
```bash
cat > /tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**

### Feature
- change
EOF

gh release create Aphrodite/vX.Y.Z --notes-file /tmp/notes.md ~/.hermes/aphrodite/aphrodite
```

## Dual-Store Guarantee (CCR_UNRESOLVED Fix)

Every proxy compress/fetch MUST also store in inline store. This prevents `[CCR_UNRESOLVED]` from race conditions or proxy eviction.

Pattern: after `_compress_via_proxy(content, ...)` succeeds with hash `h`, call `_inline_store_put(h, content)` before returning the marker.

After `_resolve_one` fetches from proxy, call `_inline_store_put(hash_val, result)` before returning.

## Version Bump — 5 Locations (v0.7.0+)

Atomization moved `_core.py` → `_core/config.py`. Auto-release now bumps:
1. `Cargo.toml` (binary)
2. `_core/config.py` (BIN_VERSION + PLUGIN_VERSION)
3. `pyproject.toml`
4. `__init__.py` docstring
5. `plugin.yaml`

Use `--minor` for feature releases, default patch for fixes.

## Session Setup Checklist

Before development:
1. Auto-expand ON: `aphrodite_catalog(mode='toc')` shows inline content
2. Proxies running: `aphrodite_stats` shows `token: on`, `cache: on`
3. Plugin symlinked to repo
4. env_passthrough set
5. Context engine ON: `[compression].context_engine = true` in `aphrodite.toml`
