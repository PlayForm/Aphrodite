# Evidence sources for real-session compression benchmarking

When a session compresses tool outputs, different sinks record different
amounts of truth. Know which one you are reading.

| Source                                                 | What it records                                                  | Verdict for marker counting                      |
| ------------------------------------------------------ | ---------------------------------------------------------------- | ------------------------------------------------ |
| CLI transcript (stdout)                                | commands + reasoning; tool-result markers are NOT rendered       | undercounts — unreliable alone                   |
| Session store (`~/.hermes/state.db`, `messages` table) | the full model context incl. tool results                        | AUTHORITATIVE — query per session id             |
| Proxy `/metrics`                                       | HTTP-path counters only (tool relay)                             | 0 for in-process dylib compression               |
| `ccr.db` (runtime home or dirs data dir)               | stored entries; the session dylib may resolve an unobserved path | best-effort only; 0 does not mean no compression |

## Querying the session store

```sql
SELECT id FROM sessions WHERE id LIKE '<YYYYMMDD>%' ORDER BY rowid;
SELECT content FROM messages WHERE session_id='<id>' AND content LIKE '%<<<CCR:%';
```

## Marker matching (Python)

```python
import re
# strip whitespace first: the 80-col CLI wraps markers across lines
norm = re.sub(r"\s+", "", transcript)
# count only REAL markers: hex hash + concrete type + numeric size
real = re.compile(r"<<<CCR:([0-9a-f]{24,})\|([a-z_]+)\|(\d+)>>>")
```

Filter OUT template/example strings: `<<<CCR:hash|type|size>>>`,
`<<<CCR:{hash}|{ct}|{size}>>>`, `<<<CCR:{hash}|text|123456>>>` (these appear
in injected directives and the model's own reasoning).

## Known db paths (macOS)

- `~/.hermes/aphrodite/ccr.db` (runtime home — the proxy's path)
- `~/Library/Application Support/aphrodite/ccr.db` (dirs data dir — often the
  session engine's path; may be months-old if the engine writes elsewhere)
