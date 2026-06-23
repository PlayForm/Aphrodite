# Security Audit: aphrodite Codebase

**Date:** 2026-06-16 **Scope:** `plugins/aphrodite/` (Python),
`crates/aphrodite/` (Rust) **Method:** READ-ONLY grep + manual code review
**Total Findings:** 15 (2 Critical, 4 High, 5 Medium, 4 Low)

---

## EXECUTIVE SUMMARY

The aphrodite codebase is designed as a **localhost-only LLM proxy** bound to
127.0.0.1 by default. Within that threat model (no external network access), the
design is reasonable. However, **several findings are critical if the proxy is
ever deployed on a reachable interface** (e.g., Docker, cloud VM, or 0.0.0.0
binding). There is **zero authentication** on any management endpoint, **no rate
limiting**, and **permissive CORS** - the proxy is wide open to anyone who can
reach it. The upstream API key is the only secret used, and it's handled
reasonably (environment variables, not CLI args), but the fallback chain that
pulls from unrelated env vars (`DEEPSEEK_API_KEY`, `HEADROOM_DEEPSEEK_KEY`) is a
latent leak vector.

---

## CRITICAL

### C-1: Zero Authentication on All Proxy Management Endpoints

**Severity:** Critical - anyone who can reach the proxy can read/write CCR data.

**Files:**

- `crates/aphrodite/src/main.rs:L98-L208` - all routes registered with no auth
  middleware
- `crates/aphrodite/src/proxy.rs:L516` - the API key is only used for upstream
  `Bearer` header, never to validate incoming requests

**Evidence:**

```
route("/retrieve", post(handle_retrieve))        ← no auth
route("/ccr/create", post(handle_ccr_create))    ← no auth
route("/ccr/{hash}", delete(handle_ccr_delete))   ← no auth
route("/stats", get(stats))                       ← no auth
route("/tool/relay", post(handle_tool_relay))     ← no auth
route("/metrics", get(metrics))                   ← no auth (comment says "intentional")
route("/*path", any(proxy_handler))               ← no auth (proxies upstream, includes auth)
```

**Risk:** The upstream API key is embedded in the proxy state, but it's only
used for outbound requests. Any inbound request to `/ccr/list`, `/ccr/create`,
or `/retrieve` is accepted unconditionally. If the proxy is bound to anything
other than 127.0.0.1, an attacker can:

- Read all compressed CCR data (conversations, tool outputs)
- Create arbitrary CCR entries (storage exhaustion)
- Delete CCR entries (data loss)
- Call the tool relay (arbitrary tool execution)

**Only exception:** `/health/upstream` checks `addr.ip().is_loopback()` (line
105).

### C-2: SSRF via Tool Relay Callback URL

**Severity:** Critical - arbitrary POST to any URL from the proxy process.

**File:** `crates/aphrodite/src/proxy.rs:L898-L907`

```rust
if let Some(cb) = &req.callback_url {
    tracker.spawn(async move {
        let result = execute_tool_relay(&state, &tool, &params).await;
        let _ = state.client.post(&cb).json(&result).send().await;  // ← SSRF
    });
}
```

**Risk:** The `callback_url` from `ToolRelayRequest` is used directly with
`state.client.post(&cb)` with no validation or allowlist. An attacker who can
reach the proxy can:

- POST arbitrary data to internal network services (metadata endpoints, internal
  APIs)
- POST to external infrastructure (data exfiltration)
- Use the proxy as an HTTP POST amplification vector

---

## HIGH

### H-1: No Rate Limiting

**Severity:** High - DoS amplification.

**Files:** `crates/aphrodite/src/*.rs`, `plugins/aphrodite/*.py`

**Evidence:** `grep -rn "rate_limit\|throttle\|too_many" crates/` returns **zero
results**. There is no rate limiting, throttling, connection limiting, or
request queuing anywhere in the codebase. An attacker can:

- Flood `/ccr/create` with large payloads (memory/storage exhaustion)
- Flood `/*path` (proxy handler) to burn upstream API quota and latency
- Exhaust the tokio async task pool via tool relay async callbacks

### H-2: No Input Size Validation on /ccr/create

**Severity:** High - memory/storage DoS.

**File:** `crates/aphrodite/src/proxy.rs:L308-L314`

```rust
pub struct CcrCreateRequest {
    pub content: String,     // ← no size limit
    pub key: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub tags: Option<Vec<String>>,
}
```

**Risk:** `content` is a `String` with no limit. An attacker can POST a
multi-gigabyte string, which will be:

1. Deserialized by serde_json (memory allocation)
2. Hashed by `compute_key()` (full content read)
3. Stored in SQLite or in-memory CCR (storage exhaustion)
4. Optionally sent as a notification POST body (amplification)

### H-3: API Key Leak Through Environment Fallback Chain

**Severity:** High - credential leakage across applications.

**File:** `crates/aphrodite/src/config.rs:L155`

```rust
let key: String = cfg.api_key.clone()
    .or_else(|| d.and_then(|d| d.api_key.clone()))
    .or_else(|| std::env::var("APHRODITE_API_KEY").ok())
    .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())      // ← pulls unrelated env var
    .or_else(|| std::env::var("HEADROOM_DEEPSEEK_KEY").ok())  // ← pulls unrelated env var
    .unwrap_or_default();
```

**Risk:** The fallback chain reads `DEEPSEEK_API_KEY` and
`HEADROOM_DEEPSEEK_KEY` from the global environment. If a user has these set for
a different application, aphrodite silently inherits them. The key is stored in
the proxy's `AppState` (a `Secret` type), exposed via `/stats` JSON, and logged
in startup message (line 88-96 logs `api_url` and `model` but NOT the key -
though if logging is later extended to include it, it's here). Also: the key is
sent in the `Authorization` header on every upstream request, visible in
dev-mode header logging.

### H-4: Unrestricted Tool Relay - Remote Tool Execution

**Severity:** High - arbitrary code execution via tool relay.

**File:** `crates/aphrodite/src/proxy.rs:L891-L915`

```rust
pub async fn handle_tool_relay(...) -> impl IntoResponse {
    match execute_tool_relay(&state, &req.tool, &req.params).await {
```

**Risk:** The tool relay accepts arbitrary tool names and parameters from any
caller (no auth). Currently only 3 tools are implemented (`aphrodite_retrieve`,
`aphrodite_compress`, `aphrodite_list`), and unknown tools return an error.
However, the architecture allows adding more relay tools, and there's no guard
against future additions. The async callback spawns a `TrackerTask` which runs
even if the HTTP response has already been sent - no cancellation mechanism
exists for long-running callbacks.

---

## MEDIUM

### M-1: Permissive CORS Configuration

**Severity:** Medium - unnecessary exposure if bound to non-loopback.

**File:** `crates/aphrodite/src/main.rs:L207`

```rust
.layer(CorsLayer::permissive())
```

**Risk:** `CorsLayer::permissive()` allows any origin, any method, any header.
This adds no value on 127.0.0.1 (where CORS is irrelevant) but is dangerous if
deployed behind a load balancer or reverse proxy without proper network
segmentation. A web page loaded from a malicious origin could make API calls to
the proxy if the user visits it while on the same network.

### M-2: Cache Poisoning via FNV-1a Hash Collisions

**Severity:** Medium - deterministic hash could be pre-imaged.

**File:** `crates/aphrodite/src/proxy.rs:L426-L434`

```rust
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    let mut hash = FNV_OFFSET;
    for &b in bytes { hash ^= b as u64; hash = hash.wrapping_mul(FNV_PRIME); }
    hash
}
```

**Risk:** The LLM response cache (line 160) uses a 64-bit FNV-1a hash as its
key. FNV-1a is not cryptographic; collisions can be constructed with ~2^32
operations. An attacker who can observe response timing or cache behavior could
craft colliding request bodies to poison the cache for another user's chat
completions. However, the cache is LRU (128 entries, per-process) and reset on
restart, limiting practical exploitability.

### M-3: Hardcoded Test API Keys

**Severity:** Medium - test code leak, sets bad pattern.

**File:** `crates/aphrodite/src/proxy.rs:L1125, L1218`

```rust
api_key: "test".into(),
```

**Risk:** Test code uses the literal string `"test"` as the API key. While this
is only in `#[cfg(test)]` blocks, it sets a pattern that could be copied into
production code. More importantly, the test state builder (`test_state()`)
clones this key into the `Secret` type, so any assertion that accidentally
prints or logs the state would expose it.

### M-4: Missing Hash Format Validation on /ccr/{hash} DELETE

**Severity:** Medium - unexpected input accepted.

**File:** `crates/aphrodite/src/proxy.rs:L1042`

```rust
pub async fn handle_ccr_delete(
    axum::extract::Path(hash): axum::extract::Path<String>,
) {
    // hash is a raw String with no validation
```

**Risk:** While CCR hashes are SHA-256 hex (64 chars), the DELETE endpoint
accepts any string. If the underlying `CcrStore` implementation (SQLite) doesn't
sanitize this input, it could be an injection vector into SQL queries. The
current SQLite store uses parameterized queries, but the validation gate should
be explicit.

### M-5: Arbitrarily Large Tags List in /ccr/create

**Severity:** Medium - unbounded memory allocation.

**File:** `crates/aphrodite/src/proxy.rs:L308-L314`

```rust
pub struct CcrCreateRequest {
    pub tags: Option<Vec<String>>,  // ← no size limit on Vec or String elements
}
```

**Risk:** An attacker can send a JSON payload with millions of tag entries or
individual tag strings megabytes in size. This is deserialized into memory
before any validation occurs.

---

## LOW

### L-1: PID File Write Race Condition

**Severity:** Low - local symlink attack with limited impact.

**File:** `plugins/aphrodite/_proxy.py:L182`

```python
Path(os.path.join(BINARY_DIR, f"proxy-{name}.pid")).write_text(str(proc.pid))
```

**Risk:** PID file is written without atomic operation (`os.replace`) and
without checking for existing symlinks. On a multi-user system, a local attacker
could pre-create a symlink at the PID path to redirect the write. `BINARY_DIR`
is `~/.hermes/aphrodite/` so this requires the attacker to already control that
directory.

### L-2: Binary Download Lacks TLS Pin

**Severity:** Low - depends on CA trust store.

**File:** `plugins/aphrodite/_binary.py:L60`

```python
with urllib.request.urlopen(download_url, timeout=30) as r:
```

**Risk:** Download uses Python's `urllib.request` which relies on system CA
certificates. No certificate pinning. If the user's CA trust store is
compromised or MITM SSL inspection is active (corporate proxies), the binary
could be replaced. Mitigated by magic-byte validation (line 71) which rejects
non-executable content.

### L-3: Clear-Text HTTP on Loopback

**Severity:** Low - local-only traffic, no encryption.

**File:** `plugins/aphrodite/_tools.py:L68`

```python
f"http://127.0.0.1:{target}/ccr/create"
```

**Risk:** All traffic between the Hermes Python plugin and the Rust proxy is
plain HTTP on 127.0.0.1. Acceptable for loopback, but if the proxy is bound to a
non-loopback address, the API key in `Authorization` headers and all CCR content
travels in cleartext.

### L-4: SQLite Database File Default Permissions

**Severity:** Low - world-readable by default.

**File:** `crates/aphrodite/src/proxy.rs:L349`

```rust
SqliteCcrStore::open(&db_path, cli.ccr_ttl_seconds)
```

**Risk:** No explicit `PRAGMA` or file permission setting on the SQLite CCR
database. Default location is `~/.local/share/aphrodite/ccr.db` (or
`/tmp/aphrodite/ccr.db`). World-readable on multi-user systems vs. `0o600`.

---

## SUMMARY

| Severity     | Count  | Key Issues                                                                                         |
| ------------ | ------ | -------------------------------------------------------------------------------------------------- |
| **Critical** | 2      | No auth on any endpoint; SSRF via tool relay callback                                              |
| **High**     | 4      | No rate limiting; no input size limit; env var leak chain; unrestricted tool relay                 |
| **Medium**   | 5      | Permissive CORS; FNV cache collisions; hardcoded test key; missing hash validation; unbounded tags |
| **Low**      | 4      | PID race; no TLS pin; cleartext loopback; DB permissions                                           |
| **Total**    | **15** |                                                                                                    |

### Key Recommendations

1. **Bind to 127.0.0.1 only** - already the default; enforce at the config level
   with a warning if a non-loopback address is used.
2. **Add auth middleware** - a shared-secret header check on all non-/health
   routes (reuse the same `api_key` as a bearer token).
3. **Validate/cap callback URL** - reject IP-literal callback URLs, require
   HTTPS, or allowlist known Hermes endpoints.
4. **Cap `content` size** in `CcrCreateRequest` to e.g. 10MB at deserialization
   boundary.
5. **Add rate limiting** - a simple semaphore or token bucket per-IP on
   `/ccr/create` and `/*path`.
6. **Remove env var fallback chain** - only read `APHRODITE_API_KEY`, not
   `DEEPSEEK_API_KEY` or `HEADROOM_DEEPSEEK_KEY`.
7. **Sanitize hash parameter** - reject non-hex hashes at the API boundary with
   a regex check `^[0-9a-f]{16,64}$`.
8. **Trim `_permissive` CORS** - use
   `CorsLayer::new().allow_origin(AllowOrigin::predicate(|_, _| false))` for
   local-only or limit to specific origins.
