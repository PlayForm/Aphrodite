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
    pub limit: usize,               // Max lines (0 = no limit)
}
```

## Response

### Success (200)

```json
{
	"found": true,
	"content": "original content...",
	"source": "ccr",
	"error": null
}
```

### Not Found (404)

```json
{
	"found": false,
	"content": null,
	"source": "none",
	"error": "CCR entry not found: abc123..."
}
```

### Bad Request (400)

```json
{
	"found": false,
	"content": null,
	"source": "none",
	"error": "`hash` required"
}
```

### Pagination Out of Range (400)

```json
{
	"found": false,
	"content": "[offset 500 out of range; document has 42 lines]",
	"source": "ccr",
	"error": null
}
```

### Decompression Error (500)

```json
{
	"found": false,
	"content": null,
	"source": "ccr",
	"error": "decompression failed"
}
```

### Response Schema

```rust
pub struct RetrieveResponse {
    pub found: bool,
    pub content: Option<String>,
    pub source: String,           // "ccr", "inline", "none"
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
| ------------ | ------------------------------------------ |
| Matching     | Case-insensitive substring match per line |
| Query length | Truncated to 512 chars                    |
| No matches   | Returns a descriptive placeholder          |

## Pagination

```rust
if req.limit > 0 {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let start = req.offset.min(total);
    let end = (start + req.limit).min(total);
    content = lines[start..end].join("\n");
    if start > 0 || end < total {
        content = format!("[lines {}-{}/{}]\n{}", start + 1, end, total, content);
    }
}
```

## Zstd Decompression

```rust
if content.as_bytes().starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
    match zstd::decode_all(content.as_bytes()) {
        Ok(decompressed) => {
            content = String::from_utf8_lossy(&decompressed).to_string();
        },
        Err(e) => {
            return 500 with "decompression failed"
        }
    }
}
```

Magic bytes `0x28 0xB5 0x2F 0xFD` identify zstd-compressed frames stored by CCR
backends.

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
