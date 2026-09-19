# CCR Management Endpoints

These loopback-only endpoints let external tools and scripts create, list, and
delete compressed content entries in the CCR store directly, and hot-reload
the compression configuration. The Hermes plugin itself compresses and
retrieves in-process through its dylib bindings - these HTTP routes are for
anything outside the plugin process.

## Authentication

All four routes are loopback only, and require `Authorization: Bearer <token>`
when `APHRODITE_MGMT_TOKEN` is set (unset = any loopback caller, back-compat
with a one-time startup warning). This management-route auth closes a real
gap: a hostile local page can issue a CORS "simple request" that lands as a
write (seeding CCR entries) even though it cannot read the reply. Note this
token is distinct from `notify_key` below, which authenticates the outbound
notification callback.

## POST /ccr/create

Creates a new CCR entry. Accepts a JSON body or raw octet-stream bytes.

### JSON Mode

**Content-Type:** `application/json`

**Request:**

```json
{
	"content": "string content to store",
	"key": "optional_custom_hash",
	"ttl_seconds": 3600,
	"tags": ["tag1", "tag2"]
}
```

`key` overrides the content hash; `ttl_seconds` defaults to 3600; `tags` is
optional. With no `key`, the hash is computed from the content itself.

**Response:**

```json
{
	"hash": "abc123def456...",
	"token_savings_ratio": 2.5,
	"original_size": 100,
	"compressed_size": 40,
	"marker_size": 40
}
```

`compressed_size`/`marker_size` are the bare hash length - this endpoint's
wire contract is the hash, not a rendered marker. `token_savings_ratio` is
`original_size / hash_length` (1.0 for empty content).

### Octet-Stream Mode

**Content-Type:** `application/octet-stream`

Raw UTF-8 bytes as the body; the hash is computed from the content. Response
is the same JSON schema as JSON mode.

### Notification

If `notify_url` is configured, an async POST fires on success:

```json
{
	"event": "ccr_created",
	"hash": "...",
	"created_at": 1234567890,
	"ttl": 3600,
	"tags": ["tag1"]
}
```

| Property | Value                                        |
| -------- | -------------------------------------------- |
| Auth     | Bearer token via `notify_key`                |
| Timeout  | 5s                                           |
| Tracking | `notify_success` / `notify_failure` counters |

### Errors

| Status | Condition                          |
| ------ | ---------------------------------- |
| 400    | Invalid JSON body                  |
| 400    | Invalid UTF-8 in octet-stream body |
| 500    | Backend store write failed         |
| 503    | CCR not enabled                    |

A 503 (rather than a fabricated success) is returned when the CCR store is
disabled - e.g. token mode with `--no-ccr-marker` - so callers never receive a
hash that would 404 on a later `/retrieve`.

### Types

```rust
// Request
pub struct CcrCreateRequest {
    pub content: String,
    pub key: Option<String>,         // custom hash override
    pub ttl_seconds: Option<u64>,
    pub tags: Option<Vec<String>>,
}

// Response
pub struct CcrCreateResponse {
    pub hash: String,
    pub token_savings_ratio: f64,
    pub original_size: usize,
    pub compressed_size: usize,
    pub marker_size: usize,
}

// Notification
pub struct CcrNotification {
    pub event: String,               // "ccr_created"
    pub hash: String,
    pub created_at: u64,             // unix seconds
    pub ttl: u64,
    pub tags: Vec<String>,
}
```

## GET /ccr/list

Returns the CCR entry count and backend info (no listing of individual
entries).

### Response (CCR enabled)

```json
{
	"entries": 42,
	"backend": "sqlite",
	"mode": "token"
}
```

### Response (CCR disabled)

```json
{
	"entries": 0,
	"message": "CCR not enabled"
}
```

| Field     | Description                                           |
| --------- | ----------------------------------------------------- |
| `entries` | Number of live entries in the CCR store               |
| `backend` | `"sqlite"` (token mode) or `"in_memory"` (cache mode) |
| `mode`    | `"token"` or `"cache"`                                |

## DELETE /ccr/{hash}

Deletes a specific CCR entry by hash.

### Response (200 OK)

```json
{
	"deleted": true,
	"hash": "abc123..."
}
```

### Response (404 Not Found)

```json
{
	"deleted": false,
	"hash": "abc123...",
	"error": "not found"
}
```

### Response (503 Service Unavailable)

```json
{
	"error": "CCR not enabled"
}
```

## POST /reload

Hot-reloads `aphrodite.toml` (or `APHRODITE_CONFIG_PATH`) and applies the
`[compression]` thresholds to the live proxy state - cache/token/inline
thresholds and the code multiplier. The `[previews] preview_max_chars` cap is
kept in sync too. Other `[compression]` keys are echoed for visibility but not
applied by the proxy.

### Response (200 OK)

```json
{
	"reloaded": true,
	"applied": true,
	"config": "aphrodite.toml",
	"compression": {
		"tool_threshold_cache": 2000,
		"tool_threshold_token": 3000,
		"inline_threshold": 500,
		"code_multiplier": 2.0
	},
	"parsed_only": {
		"auto_expand": null,
		"auto_expand_limit": null,
		"terminal_threshold": null,
		"engine_threshold_pct": null,
		"catalog_mode": null
	}
}
```

### Response (500 Internal Server Error)

```json
{
	"error": "failed to reload: <parse error>"
}
```
