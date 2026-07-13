# Rhai Scripting System - Aphrodite Plugin Architecture

Status: removed (2026-07-13) — scripting.rs deleted (report 01-T3/06-T10).
Was compile-broken, zero call sites.

## Overview

Users write `.rhai` micro-scripts that inject into aphrodite's runtime at
specific hook points. Scripts live-reload on change. Feature-gated behind
`--scripting` flag + `APHRODITE_SCRIPTING=1`.

## Hook Points (5)

| Hook          | Trigger                          | Script receives                             | Returns                        |
| ------------- | -------------------------------- | ------------------------------------------- | ------------------------------ |
| `on_compress` | Content about to be compressed   | `content, content_type, size`               | Modified content (or original) |
| `on_marker`   | CCR marker being built           | `hash, content_type, metadata_map, preview` | Modified metadata/preview      |
| `on_retrieve` | Content being retrieved from CCR | `hash, content, content_type`               | Modified content (or original) |
| `on_request`  | Request being forwarded upstream | `method, path, headers_map`                 | Modified headers               |
| `on_response` | Response received from upstream  | `status, body, content_type`                | Modified body                  |

## Script Locations

Scripts are loaded from (in order):

1. `~/.hermes/aphrodite/scripts/*.rhai` - user-level
2. `./scripts/aphrodite/*.rhai` - project-level
3. `aphrodite.toml` inline `[scripting]` blocks - config-level

## Live Reload

File watcher checks mtime every 500ms. Changed scripts are recompiled on next
hook invocation. No restart needed.

## Feature Gate

- CLI: `aphrodite --scripting`
- Env: `APHRODITE_SCRIPTING=1`
- Config: `aphrodite.toml` → `[scripting] enabled = true`
- Default: OFF (zero overhead when disabled)

## Example Script

```rhai
// ~/.hermes/aphrodite/scripts/code-filter.rhai
// Strip doc comments from Rust code before compression

fn on_compress(content, content_type, size) {
    if content_type != "code_rust" {
        return content;  // pass through
    }
    // Remove lines starting with ///
    let lines = content.split('\n');
    let filtered = [];
    for line in lines {
        if !line.trim().starts_with("///") {
            filtered.push(line);
        }
    }
    filtered.join('\n')
}
```

## Implementation Plan

### Phase 1: Engine (crates/aphrodite/src/scripting.rs)

- Rhai engine initialization
- Script discovery (glob \*.rhai)
- Compilation + caching
- Live reload (mtime check)
- Hook dispatch

### Phase 2: Integration

- Wire into compress_chat_completion (on_compress, on_marker)
- Wire into proxy_handler (on_request, on_response)
- Wire into retrieve handler (on_retrieve)

### Phase 3: Feature Gate

- CLI flag (`--scripting`)
- Config field
- Env var
- Zero-overhead when disabled (feature flag in Cargo.toml)

### Phase 4: Admin UI

- Script management API endpoints
- Script validation + testing
- Hot reload trigger

## Dependencies

- `rhai = "1"` - embedded scripting
- Feature flag: `scripting` in Cargo.toml

## Version

Target: v0.6.0 (minor bump for new feature)
