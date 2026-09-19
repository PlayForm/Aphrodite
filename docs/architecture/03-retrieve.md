# Retrieve & Recursive Marker Resolution

Two related paths turn a `<<<CCR:hash|type|size>>>` marker back into content: the HTTP `/retrieve` endpoint (inline_ccr → CcrStore, query filter + pagination, byte-exact round-trip) and the in-process recursive resolver `resolve::expand` (nested markers, depth limit, cycle-safe, never writes back).

## /retrieve HTTP path

```mermaid
sequenceDiagram
    autonumber
    participant C as caller (Hermes / curl)
    participant HR as handle_retrieve
    participant N as marker::normalize_hash
    participant IL as inline_ccr LRU (proxy AppState)
    participant CCR as CcrStore (sqlite/in-memory)
    participant F as filter_content
    participant P as paginate

    C->>HR: POST /retrieve {hash, query?, offset?, limit?}
    HR->>N: normalize_hash(hash)  (strip |type|size suffix, trim)
    alt hash missing
        HR-->>C: 400 {found:false, error:"`hash` required"}
    end
    HR->>IL: inline_ccr.get(hash)
    alt inline HIT
        IL-->>HR: content  (inline_ccr_hits++, ccr_hits++)
    else inline miss
        HR->>CCR: ccr_get(hash).await  (inline_ccr_misses++)
        alt backend HIT
            CCR-->>HR: content (ccr_hits++)
        else miss / no backend
            HR-->>C: 404 {found:false, error:"CCR entry not found"}
        end
    end
    HR->>F: filter_content(content, query)  (case-insensitive, query capped 512 chars)
    HR->>P: paginate(content, offset, limit)
    alt offset >= total (non-empty doc)
        P-->>HR: Err → 400 "[offset N out of range]"
    else full-document window
        P-->>HR: (content verbatim, truncated=false)  ← byte-exact, keeps trailing \n
    else partial window
        P-->>HR: ("[lines a-b/total]\n…", truncated=true)
    end
    HR-->>C: 200 {found:true, content, source:"ccr", truncated}
```

Pagination facts: an explicit nonzero `limit` clamps to a 10,000-line server cap; `limit == 0` is not capped and returns the full document. An empty stored document (`content: ""`) is a valid zero-line result, not an out-of-range offset. A full-window retrieval skips the lossy `lines()/join()` round-trip entirely, so the returned bytes hash back to the marker's own hash.

## Recursive resolution (resolve::expand)

```mermaid
flowchart TD
    A["expand(state, hash)"] --> B["resolve_recursive(depth=0)"]
    B --> C{"hash in visited?"}
    C -->|yes cycle| D["return resolved.get(hash) - cached pre-expansion value"]
    C -->|no| E["visited.push(hash)"]
    E --> F{"depth >= RECURSIVE_DEPTH (5)?"}
    F -->|yes| G["resolve_one(state,hash) - RAW leaf content, no further expand"]
    F -->|no| H{"resolved cache hit?"}
    H -->|yes| I["return cached"]
    H -->|no| J["content = resolve_one(hash)?  (None → propagate)"]
    J --> K["resolved.insert(hash, content)"]
    K --> L["find_markers(content) - scan &lt;&lt;&lt;CCR:…&gt;&gt;&gt;"]
    L --> M{"nested markers?"}
    M -->|none| N["return content"]
    M -->|yes| O["for each nested: recurse(depth+1) or reuse cache"]
    O --> P["replace marker text with resolved value"]
    P --> Q{"nested unresolved?"}
    Q -->|yes| R["LEAVE original marker text intact - may heal later"]
    Q -->|no| S["substitute"]
    S --> T["return result - NOT written back over hash (content-address invariant)"]
    R --> T

    subgraph resolve_one["resolve_one"]
      RA["normalize_hash"] --> RB{"i: prefix?"}
      RB -->|yes| RC["inline_store_get (inline-only)"]
      RB -->|no| RD["inline_store_get(hash)"]
    end
```

`find_markers` walks `<<<CCR:` … `>>>` pairs, tolerating unclosed prefixes without panicking. `marker::extract_hashes` additionally recognizes `[CCR:…]` and the `⫷…⫸` glyph forms via a `LazyLock` regex whose hash class is anchored to `[0-9a-fA-F:i]{6,64}` (never crossing a newline).

## Call sites

| Concern                                                 | Module                             |
| ------------------------------------------------------- | ---------------------------------- |
| `handle_retrieve`                                       | `crates/aphrodite/src/retrieve.rs` |
| `filter_content` / `paginate`                           | `crates/aphrodite/src/retrieve.rs` |
| `resolve::expand` / `resolve_recursive` / `resolve_one` | `crates/aphrodite/src/resolve.rs`  |
| `find_markers` / `parse_marker_hash`                    | `crates/aphrodite/src/resolve.rs`  |
| `normalize_hash` / `extract_hashes` (HASH_RE)           | `crates/aphrodite/src/marker.rs`   |
