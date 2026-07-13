# CCR Hinting System - Lightweight LLM Pipeline Control

Status: design-only (not implemented)

## Philosophy

Hints are **advisory signals** the LLM can pass alongside tool calls. They bias
the pipeline without controlling it. The system interprets hints, the LLM
doesn't need to learn a vocabulary.

**Hints vs Directives:**

- Directives: `{extract: ["fn","struct"], preview_depth: 5, filter: "pub"}`
- Hints: `"show signatures"` or `"code_rust"` or `"compact"`

## Hint Format

A single string passed via `_ccr_hint` parameter:

```
aphrodite_retrieve(hash="abc123", _ccr_hint="structure")
aphrodite_compress(content="...", _ccr_hint="code_rust")
read_file(path="proxy.rs", _ccr_hint="keep_signatures")
```

## Hint Types

| Hint                             | Effect                                       |
| -------------------------------- | -------------------------------------------- |
| `structure`                      | Return only structural metadata, no content  |
| `preview`                        | Return preview + structure, not full content |
| `full`                           | Return everything (default)                  |
| `compact`                        | Use compact template (single line)           |
| `code_rust`, `code_python`, etc. | Override content type detection              |
| `keep_signatures`                | Don't compress function/class signatures     |
| `keep_imports`                   | Preserve import statements in preview        |
| `summary`                        | Return a one-line summary only               |

## Implementation

### 1. Hint parsing (src/hints.rs)

```rust
pub enum Hint {
    Structure,     // return only metadata
    Preview,       // return preview + structure
    Full,          // return everything
    Compact,       // compact output format
    ContentType(String),  // override content type
    Keep(String),  // preserve specific elements
    Summary,       // one-line summary
}

pub fn parse_hint(s: &str) -> Hint {
    match s {
        "structure" => Hint::Structure,
        "preview" => Hint::Preview,
        "full" => Hint::Full,
        "compact" => Hint::Compact,
        "summary" => Hint::Summary,
        s if s.starts_with("keep_") => Hint::Keep(s[5..].to_string()),
        s => Hint::ContentType(s.to_string()),
    }
}
```

### 2. Apply hints to retrieval

When LLM calls `aphrodite_retrieve(hash, _ccr_hint="structure")`:

- Return only the structure line:
  `[code_rust: fns=main,helper;structs=App;ln=1989]`
- Skip the content and marker lines

### 3. Apply hints to compression

When LLM calls `aphrodite_compress(content, _ccr_hint="code_rust")`:

- Skip auto-detection, use `code_rust` type
- Apply code-specific structure extraction

### 4. Hint passthrough in markers

Hints are embedded in the marker metadata so future retrievals know the original
intent:

```
<<<CCR:hash|code_rust|4832|hint=structure>>
```

## Why Hints Over Directives

1. **Zero learning curve** - LLM says what it wants in natural terms
2. **System interprets** - hints are advisory, pipeline decides how to apply
3. **Self-documenting** - hint string IS the documentation
4. **Extensible** - new hints don't break old ones
5. **Safe** - hints can only ADD information, never hide it
