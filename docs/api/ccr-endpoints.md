# CCR Management Endpoints

Origin: Programmatic CCR create, list, and delete  -  allowing the Python plugin, Hermes agent, and external tools to manage compressed content entries directly.

Source of truth: `crates/aphrodite/src/proxy.rs:handle_ccr_create()` (line 1635), `handle_ccr_list()` (line 1751), `handle_ccr_delete()` (line 1771)

## POST /ccr/create

Creates a new CCR entry. Supports both JSON and raw octet-stream bodies.

### Access
Loopback only.

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

### Octet-Stream Mode

**Content-Type:** `application/octet-stream`

**Request:** Raw UTF-8 bytes as body.

**Response:** Same JSON schema as JSON mode.

### Notification
If `notify_url` configured: async POST with:
```json
{
    "event": "ccr_created",
    "hash": "...",
    "created_at": 1234567890,
    "ttl": 3600,
    "tags": ["tag1"]
}
```
Auth: Bearer token via `notify_key`. Timeout: 5s. Tracked via `notify_success`/`notify_failure`.

### Errors

| Status | Condition |
|--------|-----------|
| 400 | Invalid JSON body |
| 400 | Invalid UTF-8 in octet-stream body |

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

Returns CCR entry count and backend info.

### Access
Loopback only.

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

### Fields

| Field | Description |
|-------|-------------|
| `entries` | Number of live entries in CCR store |
| `backend` | `"sqlite"` (token mode) or `"in_memory"` (cache mode) |
| `mode` | `"token"` or `"cache"` |

## DELETE /ccr/{hash}

Deletes a specific CCR entry by hash.

### Access
Loopback only.

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

### Implementation
```rust
pub async fn handle_ccr_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> impl IntoResponse {
    match &state.ccr {
        Some(ccr) => {
            let existed = ccr_del(ccr, &hash).await;
            if existed {
                (StatusCode::OK, Json({"deleted": true, "hash": hash}))
            } else {
                (StatusCode::NOT_FOUND, Json({"deleted": false, "hash": hash, "error": "not found"}))
            }
        },
        None => (StatusCode::SERVICE_UNAVAILABLE, Json({"error": "CCR not enabled"})),
    }
}
```

## Python Plugin Usage

The Python plugin uses these endpoints extensively:

### _compress_via_proxy (_marker/marker.py:228)
```
POST /ccr/create with Content-Type: application/octet-stream
```

### _compress_handler (_tools.py:98)
```
POST /ccr/create → get hash → mirror in inline store
```

### _store_conversation_turn (_hooks/transform.py:476)
```
POST /ccr/create with turn data → store in _conv_index
```

### Context engine (_engine.py:210)
```
POST /ccr/create with packed messages → get hash → build marker
```
