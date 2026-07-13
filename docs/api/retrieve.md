# Retrieve Endpoint

Resolves a CCR hash back to its original content, with optional query
filtering and pagination. It's used by the LLM agent whenever
`aphrodite_retrieve` is called.

## Endpoint

```
POST /retrieve
```

## Access

Loopback only.

## Request

```json
{
	"hash": "abc123...",
	"query": "optional filter string",
	"offset": 0,
	"limit": 100
}
```

### Request Schema

```rust
pub struct RetrieveRequest {
    pub hash: Option<String>,       // Required
    pub query: Option<String>,      // Optional: case-insensitive line filter
    #[serde(default)]
    pub offset: usize,              // 0-based line offset for pagination
    #[serde(default)]
    pub limit: usize,               // Max lines (0 = server default cap: 10,000)
}
```

## Response

### Success (200)

```json
{
	"found": true,
	"content": "original content...",
	"source": "ccr",
	"error": null,
	"truncated": false
}
```

`truncated` is `true` when `content` is a partial window of a larger stored
document (because of `offset`/`limit`, or because `limit` hit the 10,000-line
cap) - see [Pagination](#pagination).

### Not Found (404)

```json
{
	"found": false,
	"content": null,
	"source": "none",
	"error": "CCR entry not found: abc123...",
	"truncated": false
}
```

### Bad Request (400)

```json
{
	"found": false,
	"content": null,
	"source": "none",
	"error": "`hash` required",
	"truncated": false
}
```

### Pagination Out of Range (400)

```json
{
	"found": false,
	"content": "[offset 500 out of range; document has 42 lines]",
	"source": "ccr",
	"error": null,
	"truncated": false
}
```

### Response Schema

```rust
pub struct RetrieveResponse {
    pub found: bool,
    pub content: Option<String>,
    pub source: String,           // "ccr", "inline", "none"
    pub truncated: bool,          // true if content is a partial window
    pub error: Option<String>,
}
```

## Retrieve Flow

```
1. Validate hash (required)
2. Check inline_ccr (lock dropped before any .await):
   a. Hit → inline_ccr_hits++, ccr_hits++, return content
   b. Miss → inline_ccr_misses++, fall through
3. Check CCR backend:
   a. Hit → ccr_hits++
   b. Miss → ccr_misses++, return 404
4. Decompress zstd if magic bytes (0x28, 0xB5, 0x2F, 0xFD):
   a. zstd::decode_all()
   b. Fail → return 500
5. Apply query filter (case-insensitive, max 512 chars):
   a. If no matches: "[no lines matching "query" in N lines]"
   b. Otherwise: filtered lines
6. Apply pagination (offset + limit):
   a. If offset >= total lines: 400 out-of-range
   b. Slice lines[start..end]
   c. Prepend: "[lines {start}-{end}/{total}]" when paginated
7. Return 200 with {found: true, content, source: "ccr"}
```

## Query Filter

```rust
fn filter_content<'a>(content: &'a str, query: Option<&str>) -> Cow<'a, str> {
    match query {
        Some(q) if !q.is_empty() => {
            let q = if q.len() > 512 { &q[..512] } else { q };  // truncate to 512
            let filtered: Vec<&str> = content
                .lines()
                .filter(|line| line.to_lowercase().contains(&q.to_lowercase()))
                .collect();
            if filtered.is_empty() {
                Cow::Owned(format!("[no lines matching {:?} in {} lines]", q, content.lines().count()))
            } else {
                Cow::Owned(filtered.join("\n"))
            }
        },
        _ => Cow::Borrowed(content),
    }
}
```

| Behavior     | Detail                                    |
| ------------ | ----------------------------------------- |
| Matching     | Case-insensitive substring match per line |
| Query length | Truncated to 512 chars                    |
| No matches   | Returns a descriptive placeholder         |

## Pagination

`limit: 0` does not mean unlimited - it is clamped to a 10,000-line server
default cap, same as any `limit` above 10,000. When the returned window
doesn't cover the whole document (because of `offset`, `limit`, or the cap),
a `[lines a-b/total]` header is prepended to `content` so the caller can tell
a truncated result from a genuinely short document without guessing.

## Source Tracking

| source value | Meaning                                                 |
| ------------ | ------------------------------------------------------- |
| `"ccr"`      | Found in CCR store (SQLite or in-memory)                |
| `"inline"`   | Would be set for inline store (currently "ccr" is used) |
| `"none"`     | Not found (error response)                              |

## Production Note

The inline_ccr lock is dropped BEFORE any `.await` to avoid `!Send MutexGuard`
crossing await points. The entire check-and-resolve for inline is scoped in a
block.
