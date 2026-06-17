---
name: aphrodite-release-workflow
description: "Auto-release, version sync, CCR format, budget tuning, and new feature workflow for aphrodite v0.5.69+."
version: 1.0.0
platforms: [macos]
---

# Aphrodite Release & Feature Workflow

Post v0.5.69 patterns that every agent must follow.

## Auto-Release

```bash
GIT_EDITOR=true scripts/auto-release.sh "descriptive message"
```
Handles: stage → commit → bump version → cargo build → cargo test → tag → push to `Source` remote.
One command. No editor prompts.

Remote is `Source` (ssh), NOT `origin`. If push fails, verify remote name with `git remote -v`.

## Version Sync

4 locations must match:
1. `plugins/aphrodite/_core.py` — `BIN_VERSION` + `PLUGIN_VERSION`
2. `plugins/aphrodite/plugin.yaml` — `version:`
3. `crates/aphrodite/Cargo.toml` — `version =`
4. Auto-release script handles Cargo.toml + _core.py sync

## CCR Format (v0.5.71+)

Multi-line, editable via `format_ccr_output()` in `proxy.rs`:
```
{preview ~250 chars}
[{type}: {metadata}]
<<<CCR:{hash}|{type}|{size}>>>
```
- `;` separates key=value pairs in metadata
- `,` separates list items
- Python `_ccr_marker()` matches
- Parser unchanged

## Budget Defaults (v0.5.70+ — coding-tuned)

| Setting | Old | New |
|---------|-----|-----|
| ENGINE_THRESHOLD_PCT | 50 | 65 |
| ENGINE_PROTECT_FIRST | 1 | 5 |
| ENGINE_PROTECT_LAST | 1 | 5 |
| ENGINE_MIN_MSGS | 4 | 12 |
| build_output/log threshold | ÷2 | ×1 |
| budget_mult floor | 0.25 | 0.50 |

## Prometheus (v0.5.69+)

- Docker: `prom/prometheus` on `:9090`
- Config: `prometheus.yml` scrapes `:9797` + `:9798`
- 31 metrics, Prometheus naming conventions enforced (`_total` suffix)
- Scripts: `scripts/prometheus.sh start|stop|status`

## Hints (v0.5.78+)

LLM passes `_ccr_hint` in tool params — session-scoped mode switch:
- `debug` — full errors, deeper extraction
- `review` — keep imports, show diffs
- `code_rust` / `code_python` — code-aware extraction
- `compact` / `verbose` — preview sizing
- Hints compose additively across turns

## Rhai Scripting (v0.5.75+)

Feature-gated: `--features scripting` + `APHRODITE_SCRIPTING=1`
Scripts: `scripts/aphrodite/*.rhai` or `~/.hermes/aphrodite/scripts/*.rhai`
3 hooks: `on_compress`, `on_marker`, `on_retrieve`
Live-reload on file change.

## GitHub Releases

**CRITICAL: Never use inline backtick-quoted text with `gh release create --notes`.** The shell interprets backticks as command substitution, capturing Hermes TUI output into the release body. Always use `--notes-file` with a temp file.

### Release notes format

Every release MUST include:
1. Compare link to previous tag
2. Section headers (###) for each feature area
3. Bullet points with specific changes
4. Code examples where relevant
5. Before/after comparisons where applicable

### Creating a release

```bash
# Write notes to temp file (heredoc prevents shell interpretation)
cat > /tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**

### Feature

- change one
- change two
EOF

# Create release with binary
gh release create vX.Y.Z --repo PlayForm/Aphrodite \
  --title "vX.Y.Z — description" \
  --notes-file /tmp/notes.md \
  ~/.hermes/aphrodite/aphrodite
```

### Template helper

```bash
./scripts/release-notes.sh v0.5.122 "title" "body text" > /tmp/notes.md
gh release create v0.5.122 -F /tmp/notes.md ~/.hermes/aphrodite/aphrodite
```

## Code Structure Preview (v0.5.70+)

On by default, no feature flag. `generate_metadata()` extracts:
- Rust: fns, structs, impls, traits
- Python: fns, classes, imports, decorators
- Go/JS: fns
- Generic: sigs (fn/def/func/class/struct)
