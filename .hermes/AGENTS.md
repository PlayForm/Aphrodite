# Aphrodite 💋 Development Context

Welcome! You're working on Aphrodite - a blazing-fast CCR compression engine
that saves millions of tokens and makes AI agents dramatically more efficient.
Every line you write here directly makes LLMs cheaper, faster, and smarter. ✨

## Project

- **Rust proxy** - LLM API with intelligent CCR compression
- **Two crates**: `crates/aphrodite` (core engine) + `crates/aphrodite-hermes` (agent integration)
- **Plugin**: `plugins/aphrodite` v2.0.6 - 13 tools, 9 skills, dylib hot-reload, dual-proxy architecture
- **Config**: `aphrodite.toml` - all tuning in one file, env-overridable
- **Binary**: `~/.hermes/aphrodite/aphrodite` - auto-downloaded, auto-updated (run `aphrodite --version` for the current version, not a hardcoded one here)

## Dev Flow - The Joyful Loop

- **Pane 0**: `cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x 'run -p aphrodite'` - instant feedback on every save
- **Pane 1**: `hermes --profile dev-aphrodite` - test in production immediately
- **Pane 2**: WezTerm MCP for scripted verification - never interrupt the main session!

Pro tip: The Rust dylib hot-reloads on mtime change - rebuild and the plugin picks
it up without restarting Hermes. Pure magic. 🪄

Watch BOTH packages, not just `aphrodite`: `-p aphrodite` alone never rebuilds
`libaphrodite_hermes.dylib` (it's a sibling package that depends on `aphrodite`,
not the other way around, so Cargo has no reason to touch it). Watching only
`aphrodite` means the proxy pane looks alive while the Hermes plugin keeps
running old code indefinitely.

## Key Paths

| What | Where |
|------|-------|
| Binary | `target/release/aphrodite` |
| Plugin | `plugins/aphrodite/` (thin Python ctypes loader → Rust dylib) |
| Environment | `~/.hermes/.env` (`APHRODITE_API_KEY`) |
| Plans | `.hermes/plans/` |
| Templates | `.hermes/RELEASE-TEMPLATE.md` |
| Maintenance | `Maintain/scripts/`, `Maintain/CHANGELOG.md` |

## Agent Team - Your Digital Colleagues

- **Main agent**: Development + testing - the tip of the spear
- **Background workers**: Research, code review, test generation - tireless helpers
- **Cron jobs**: Health checks, release builds - the unsung heroes

These agents work in parallel - delegate freely and trust the results!

## Quality Gates - Zero Tolerance for Warnings

```
ruff check plugins/aphrodite/     → 0 errors, every time
npx pyright plugins/aphrodite/    → 0 errors, every time
cargo test -p aphrodite           → all green, every time
```

We ship clean. Always have, always will.

## Profiles - Pick Your Power Level

```
aphrodite-barebone          → Minimal, just the basics
aphrodite-compress-off      → Full tools, no compression
aphrodite-compress-light    → Gentle savings
aphrodite-compress-medium   → Balanced
aphrodite-compress-aggressive → Maximum efficiency, full throttle 🚀
aphrodite-proxy-cache       → Cache-mode proxy testing
aphrodite-proxy-token       → Token-mode proxy testing
```
