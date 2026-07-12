# Centers - AI Conversation Memory Tokens

This is a roadmap page: only "v1 (current)" below is implemented (see
[Current Implementation](#current-implementation)); v2-v4 are unimplemented
design sketches, not shipped features. Don't configure against this page
expecting bucket/filesystem-style centers to exist yet.

## Vision

A **center** is a traveling memory annotation embedded in CCR markers. When the
AI first encounters content, it deposits a center - a note about what it was
thinking. Future retrievals see that center and understand the original context.

## Evolution Path

### v1 (current) - Simple String Annotation

```
_ccr_center="code_rust"     → marker shows ;center=code_rust
_ccr_center="debug"         → marker shows ;center=debug
```

The center is a single string. It annotates, it doesn't accumulate.

### v2 - Bucketed Centers

```
_ccr_center="bucket:review"  → marks file for review
_ccr_center="bucket:todo"    → adds to TODO inventory
_ccr_center="bucket:bug"     → flags as bug-related
```

Centers gain structure. Buckets categorize the intent. Multiple AIs can add to
different buckets on the same CCR entry.

### v3 - Accumulative Centers

```
Marker carries: ;center=code_rust;bucket=review,todo
```

Future AIs append to existing centers. A file marked "code_rust" by AI-1 gets
"bucket:review" added by AI-2. Centers accumulate over time, building a
collaborative annotation layer on compressed content.

### v4 - Center as File System

```
~/.hermes/aphrodite/centers/
  code_rust/     → files understood as Rust
  review/        → files needing review
  todo/          → files with pending work
  bug/           → files with known issues
  inventory/     → all indexed files
```

Centers become a lightweight virtual filesystem. CCR hashes are grouped by
center. Retrieval can filter by center. The inventory is the union of all
centers.

## Current Implementation

| Layer                      | Support                       |
| -------------------------- | ----------------------------- |
| Rust `format_ccr_output`   | `;center=X` in structure line |
| Rust `smart_marker`        | `center: Option<&str>` param  |
| Rust tool relay            | `_ccr_center` from params     |
| Python `_ccr_marker`       | `center=None` param           |
| Python `_compress_handler` | `X-Aphrodite-Center` header   |
| Python tool schema         | `_ccr_center` exposed         |

## What the LLM Sees

```
use std::sync::Arc;
use axum::{Router, extract::State};
fn main() -> anyhow::Result<()> {
[code_rust: lang=rs;fns=build_state,run_single;structs=AppState;ln=1989;center=code_rust]
<<<CCR:abc123|code_rust|67097>>>
```

The center annotation on line 2 tells the LLM: "When this was compressed, the AI
understood it as Rust code." This context travels with the marker through
retrievals, providing a persistent memory of intent.
