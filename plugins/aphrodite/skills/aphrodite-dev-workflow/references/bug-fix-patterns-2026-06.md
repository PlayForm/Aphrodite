# Bug Fix Patterns — June 2026 Session

## Proxy Retry with Header Forwarding

Don't build `req` once then retry `.send()` — `reqwest::RequestBuilder` doesn't implement Clone.
Instead, inline the retry loop and rebuild the request each attempt:

```rust
let body_vec = body.to_vec();
let mut upstream_result = Err("unreachable".to_string());
for attempt in 1..=3u32 {
    let req = state.client.request(method.clone(), &url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", state.api_key));
    let mut req = req;
    for (key, val) in headers.iter() {
        let k = key.as_str().to_lowercase();
        if k != "host" && k != "authorization" && k != "content-length" {
            if k.starts_with("x-headroom-") && k != "x-headroom-workspace" {
                continue;
            }
            req = req.header(key, val);
        }
    }
    match req.body(body_vec.clone()).send().await {
        Ok(r) => { upstream_result = Ok(r); break; }
        Err(e) => {
            if attempt < 3 {
                let ms = 100 * 2u64.pow(attempt - 1);
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            } else { upstream_result = Err(format!("{}", e)); }
        }
    }
}
```

Note: extract `content_type` from `response.headers()` BEFORE `response.bytes()` — `bytes()` consumes the response.

## Config Propagation from aphrodite.toml

`MultiConfig::resolve()` must plumb every field through from `ProxyConfig`:

```rust
pub struct ProxyConfig {
    // ... existing fields ...
    pub notify_url: Option<String>,    // was hardcoded None
    pub notify_key: Option<String>,    // was hardcoded None
    pub timeout: Option<u64>,          // was hardcoded 300
    pub max_context: Option<usize>,    // was hardcoded 1_000_000
    pub max_output: Option<usize>,     // was hardcoded 384_000
}
```

Then in `resolve()`:
```rust
notify_url: cfg.notify_url.clone(),  // not None
notify_key: cfg.notify_key.clone(),  // not None
timeout: cfg.timeout.unwrap_or(300),
max_context: cfg.max_context.unwrap_or(1_000_000),
max_output: cfg.max_output.unwrap_or(384_000),
```

## ccr_misses Double-Count Fix

In `retrieve.rs`, the unconditional `ccr_misses.fetch_add` after the `if let Some(ccr)` block fires for BOTH the "hash not found" and "CCR not enabled" cases. Fix with `else` branch:

```rust
if let Some(ccr) = &state.ccr {
    match ccr.get(&hash) {
        Some(content) => { ccr_hits += 1; return ... }
        None => { ccr_misses += 1; }
    }
} else {
    ccr_misses += 1;
}
// Remove: dangling ccr_misses += 1 (was double-counting)
```

## filter_content Empty Fallback

When `query` matches zero lines, return FULL content instead of empty string:

```rust
fn filter_content(content: &str, query: Option<&str>) -> String {
    match query {
        Some(q) if !q.is_empty() => {
            let filtered: Vec<&str> = content.lines()
                .filter(|line| line.to_lowercase().contains(&q.to_lowercase()))
                .collect();
            if filtered.is_empty() {
                content.to_string()  // fallback to full content
            } else {
                filtered.join("\n")
            }
        }
        _ => content.to_string(),
    }
}
```

## tokens_saved Tracking

Increment on compression, not just expose the counter. Two sites:
1. In `compress_chat_completion()` — after CCR store insertion.
2. In `handle_ccr_create()` — after programmatic CCR insertion (the `/ccr/create` endpoint).

```rust
// compress_chat_completion
state.tokens_saved.fetch_add((original_size - marker.len()) as u64, Ordering::Relaxed);

// handle_ccr_create
state.tokens_saved.fetch_add((original_size - hash.len()) as u64, Ordering::Relaxed);
```

Without the `handle_ccr_create` increment, programmatic CCR entries from the Python plugin
(e.g. `_store_conversation_turn`, `_pre_llm_hook` turn archive) never update `tokens_saved`,
so `aphrodite_stats` always reports 0 savings.

## Headroom Submodule Workflow

Our fork: `git@github.com:NikolaRHristov/headroom.git` (origin)
Upstream: `https://github.com/chopratejas/headroom.git` (upstream)

```bash
cd vendor/headroom
git fetch upstream
git rebase upstream/main
# make changes, commit
git push origin main
cd ../..
git add --force vendor/headroom
git commit -m "chore: update headroom submodule — ..."
```

Key fix: `is_internal_header()` in `crates/headroom-proxy/src/headers.rs` must exclude `x-headroom-workspace`:

```rust
pub fn is_internal_header(name: &HeaderName) -> bool {
    let lower = name.as_str().to_ascii_lowercase();
    lower.starts_with(INTERNAL_HEADER_PREFIX) && lower != "x-headroom-workspace"
}
```

## dev=true Must Be Disabled in Committed Config

`aphrodite.toml` should NEVER have `dev = true` committed — it enables full request/response body logging to stdout, leaking message content and API responses. Comment it out:
```toml
# dev = true  # set locally only — do not commit
```
