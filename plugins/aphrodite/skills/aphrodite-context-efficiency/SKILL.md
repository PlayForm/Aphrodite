---
name: aphrodite-context-efficiency
description:
    Techniques for minimizing token usage when working with aphrodite
    compression — audit patterns, tool selection, and compression policy
    awareness.
version: 1.0.0
platforms: [macos]
---

# Aphrodite Context Efficiency

How to keep sessions lean when aphrodite compression is active.

## Tool Selection: `search_files` over `read_file`

When scanning codebases or verifying fixes, prefer `search_files` with targeted
patterns over `read_file` with broad slices. Two reasons:

1. `search_files` output is auto-compressed into CCR markers — each match is a
   tiny `<<<CCR:hash|tool|size>>>` instead of hundreds of raw lines
2. `search_files` with specific regex patterns returns only relevant lines, not
   the entire file

Real example — verifying 18 audit fixes across proxy.rs (2182 lines):

- 3 × `read_file` = ~1500 raw lines → 40K+ tokens
- 10 × `search_files` with patterns → ~5K tokens compressed

Even after the v0.5.104 `_ESSENTIAL_TOOLS` refactor (which makes `read_file`
compressible), `search_files` still wins because it returns only matching
content, not unrelated code.

## Batch Independent Calls

Multiple tool calls that don't depend on each other MUST be dispatched in ONE
response. Hermes runs them concurrently. This applies to: `read_file`,
`search_files`, `aphrodite_prefetch`, and any other independent I/O.

Example — reading 3 files:

```
❌ Serial:  read_file(A) → wait → read_file(B) → wait → read_file(C)
✅ Batch:   read_file(A) + read_file(B) + read_file(C) in one response
```

The memory instruction `BATCH INDEPENDENT TOOL CALLS` is active — follow it.

## Compression Awareness

Understand what compresses and what doesn't:

- `aphrodite_*` tools: NEVER compressed (proxy-level protection against
  double-wrap)
- `search_files`, `terminal`: always compressed above threshold
- `read_file`, `skill_view`, `session_search`: compressed above threshold (since
  v0.5.104)

## The CCR Marker IS the Proof

**Critical efficiency principle**: When content is compressed, the returned
`<<<CCR:hash|type|size>>>` marker IS the verification. You do NOT need to
`aphrodite_retrieve` the content back just to confirm compression worked. The
hash exists, the type is correct, the size is non-zero — that's proof.

Retrieving to verify wastes:

- A full tool call round-trip
- A token cache entry for the response
- Context space in the conversation

Only retrieve when you actually need the content for downstream processing
(reading a file, analyzing an error, feeding data to another tool). Never
retrieve just to check "did it work?"

## Search-First, Retrieve-Last

`search_files` with `output_mode='content'` returns matching lines directly in
the result when output is small. Exploit this:

1. **Pattern precision**: Narrow your regex so matches are few and compact. A
   `search_files` with 3 short matches stays inline; with 50 matches it
   compresses. Target the exact symbol, not broad patterns.
2. **Read the inline match**: If `search_files` returns inline content (no CCR
   wrapper) and the matching line contains your answer — stop. You already have
   it.
3. **`files_only` for location**: When you just need to find WHERE something
   lives, use `output_mode='files_only'` — tiny output, almost never compresses.
4. **When matches ARE compressed**: Check if the match count + file paths in the
   preview snippet tell you enough. Often they do — you know which files to
   target next without retrieving.

## Config File Exclusion

Common project manifest files are compressed in nearly every session. They
rarely need retrieval because:

- Their content is predictable (version strings, dependency lists, standard
  metadata)
- `search_files` with narrow patterns extracts the one field you need without
  retrieving the whole file
- You've seen these files before — they only change during explicit version
  bumps

**Skip retrieval for these unless you're about to edit them:**

| File             | Why skip                                               |
| ---------------- | ------------------------------------------------------ |
| `Cargo.toml`     | Version + deps only; `search_files` for the one field  |
| `pyproject.toml` | Same — metadata; narrow search beats full read         |
| `package.json`   | Same pattern                                           |
| `go.mod`         | Module path + deps                                     |
| `Makefile`       | Target names; `search_files(pattern='^[a-z].*:')`      |
| `.gitignore`     | Never needs retrieval                                  |
| `CHANGELOG.md`   | Use `read_file(offset=1, limit=30)` for recent entries |

If you MUST read a config file, use `read_file` with tight `offset`/`limit`
bounds — don't retrieve a 200-line TOML just to check a version string on
line 3.

## Pitfalls

- Reading a 500-line block of a 2000-line file to check one pattern wastes 40K+
  tokens
- Running `cargo fmt` then retrieving the full diff (34KB whitespace noise)
  defeats compression
- Sequential tool calls when independent ones could batch — adds unnecessary
  turn overhead
- Retrieving `Cargo.toml` / `pyproject.toml` just to check a version —
  `search_files` inline match already has it
- Retrieving a `search_files` result when the match count + file list is
  sufficient context
- **Over-skilling**: loading skills you don't need. A terse user trigger means
  "work on the obvious task" — not "load every related skill". Load skills only
  when the task is clear
- **Duplicate calls**: never re-run a terminal, skill_view, or retrieve whose
  result you just received. The output is cached; a duplicate returns identical
  content — wasted tokens
- **Thinking spiral**: when "Σ ~N total" climbs past 100 without action, you're
  over-thinking. Make a tool call or give the user output
- **Compressed previews are data**: `[json:success,name,description...]` and
  `[tool: ]` from compressed skill_view results mean compression worked. The
  preview IS the summary — only retrieve if it hints at unknown content you need
