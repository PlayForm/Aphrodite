"""COMPRESSION_AWARE_TASK"""

from .types import Conversation, ToolCall, Turn

# ═══════════════════════════════════════════════════════════════════════════════
COMPRESSION_AWARE_TASK = Conversation(
    name="compression_aware_task",
    description="Deliberately minimize context - CCR prefetch, previews over raw output, retrieve only when needed",
    system_prompt="""You are a context-frugal assistant operating under an aggressive token budget. CCR (context compression) is available: aphrodite_prefetch stores whole files as markers, aphrodite_search returns compact hits with previews, aphrodite_catalog lists stored entries, and aphrodite_retrieve expands a marker only when its content is actually needed. Never pull raw content you do not need - prefer markers and previews, and retrieve at most the exact entries required to answer. State explicitly what you chose NOT to expand.""",
    turns=[
        # Turn 1: User requests exploration under a tight context budget
        Turn(
            role="user",
            content="""I need to understand how the Aphrodite compression pipeline classifies tool output - the content types, the thresholds, and how previews are generated. The source is at `/tmp/aphrodite-src/` (compression module under `src/compression/`). I am on a tight context budget, so treat every token as expensive: use CCR aggressively - prefetch whole files as markers, show previews instead of raw output, and retrieve only the exact entry you need. Do not dump full file contents into context.""",
        ),
        # Turn 2: Assistant prefetches into CCR and searches
        Turn(
            role="assistant",
            content="Understood - strategy: prefetch the core pipeline files into CCR as markers, search for the classification logic to find the exact entry, then retrieve only that one entry when answering.",
            tool_calls=[
                ToolCall(
                    id="call_c1",
                    name="aphrodite_prefetch",
                    arguments='{"files": ["/tmp/aphrodite-src/src/compression/classifier.rs", "/tmp/aphrodite-src/src/compression/engine.rs", "/tmp/aphrodite-src/src/compression/marker.rs", "/tmp/aphrodite-src/src/compression/store.rs"]}',
                ),
                ToolCall(
                    id="call_c2",
                    name="aphrodite_search",
                    arguments='{"query": "classify content type threshold preview", "type": "code"}',
                ),
            ],
        ),
        # Turn 3: Tool results - markers + previews, compact search hits
        Turn(
            role="tool",
            tool_call_id="call_c1",
            content="""Prefetched 4 files → CCR markers (no raw content pulled into context):

  src/compression/classifier.rs   → <<<CCR:a3f9c21e7b04d5f8a6c3e2d1b9f8472a|code|5231>>>   [preview: ContentType enum, threshold table, preview generation]
  src/compression/engine.rs       → <<<CCR:7e2b8d4f6a1c9e3b5d7f8a2c4e6b1d9f0|code|12440>>>  [preview: pipeline orchestration, classifier → marker → store]
  src/compression/marker.rs       → <<<CCR:1c4e7a9b2d5f8e0a3b6c9d1e4f7a2b8c|code|3208>>>   [preview: <<<CCR:hash|type|size>>> marker + preview templates]
  src/compression/store.rs        → <<<CCR:9d2f5a8b1c4e7d0a3b6f9c2e5a8d1b4f|code|8941>>>   [preview: hash-keyed storage, BLAKE3, TTL expiry]

Stored 4 markers: ~29.9 KB raw → ~600 B of markers/previews in context.
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_c2",
            content="""Search 'classify content type threshold preview' (type=code) - 6 hits:

  src/compression/classifier.rs:12    ContentType enum: Code, Diff, BuildOutput, Error, Json, Terminal, Git, Text
  src/compression/classifier.rs:41    fn classify(&self, output: &str) -> ContentType
  src/compression/classifier.rs:87    per-type thresholds: code=4096, diff=3072, build=2048, error=1536, terminal=1024, json=4096
  src/compression/classifier.rs:118   preview generation: first N chars + structure map (function/struct names)
  src/compression/engine.rs:203       classifier.classify() → threshold check → marker → store
  src/compression/preview.rs:15       preview_max_chars default 400

Hits 1-4 are inside the prefetched classifier.rs marker - retrieve that one entry if needed.
""",
        ),
        # Turn 4: Assistant checks catalog + stats without expanding anything
        Turn(
            role="assistant",
            content="Prefetch returned markers only (~29.9 KB of raw source stayed out of context) and search returned compact hits with line numbers. Checking the catalog and session stats to confirm what is storable.",
            tool_calls=[
                ToolCall(id="call_c3", name="aphrodite_catalog", arguments='{"mode": "compact"}'),
                ToolCall(id="call_c4", name="aphrodite_stats", arguments="{}"),
            ],
        ),
        # Turn 5: Tool results - compact catalog + stats
        Turn(
            role="tool",
            tool_call_id="call_c3",
            content="""CCR catalog (compact, 4 entries):

  # | type | size  | marker                                                              | source
  1 | code | 5231  | <<<CCR:a3f9c21e7b04d5f8a6c3e2d1b9f8472a|code|5231>>>                 | classifier.rs
  2 | code | 12440 | <<<CCR:7e2b8d4f6a1c9e3b5d7f8a2c4e6b1d9f0|code|12440>>>              | engine.rs
  3 | code | 3208  | <<<CCR:1c4e7a9b2d5f8e0a3b6c9d1e4f7a2b8c|code|3208>>>               | marker.rs
  4 | code | 8941  | <<<CCR:9d2f5a8b1c4e7d0a3b6f9c2e5a8d1b4f|code|8941>>>              | store.rs

All four entries already stored; engine.rs is the largest (12.4 KB) - left as a marker unless needed.
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_c4",
            content="""Session stats:

  entries_created:       4
  entries_retrieved:     0
  bytes_compressed:      29920
  bytes_retrieved:       0
  markers_in_context:    4
  raw_bytes_in_context:  0
  estimated_tokens_saved: ~7000
""",
        ),
        # Turn 6: User asks a specific question
        Turn(
            role="user",
            content="Good. Specifically - how does classifier.rs decide the content type at the threshold boundary, and what does a generated preview look like?",
        ),
        # Turn 7: Assistant does ONE targeted retrieve
        Turn(
            role="assistant",
            content="One targeted retrieve - the classifier entry only. engine.rs, marker.rs, and store.rs stay as markers.",
            tool_calls=[
                ToolCall(
                    id="call_c5",
                    name="aphrodite_retrieve",
                    arguments='{"hash": "a3f9c21e7b04d5f8a6c3e2d1b9f8472a"}',
                ),
            ],
        ),
        # Turn 8: Tool result - retrieved classifier.rs (large, gets CCR'd again under the cache proxy)
        Turn(
            role="tool",
            tool_call_id="call_c5",
            content="""// src/compression/classifier.rs - content-type detection for CCR
//
// The classifier is the pre-filter of the compression pipeline. It looks
// at a tool output and decides (a) which ContentType it is and (b) whether
// it is large enough to justify CCR compression. Small or "clean" outputs
// skip CCR entirely (classifier_poll pre-filter).

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    Code,
    Diff,
    BuildOutput,
    Error,
    Json,
    Terminal,
    Git,
    Text,
}

impl ContentType {
    /// Default per-type compression threshold in bytes. An output is
    /// compressed only when size >= threshold[type].
    pub fn default_threshold(self) -> usize {
        match self {
            ContentType::Code => 4096,
            ContentType::Diff => 3072,
            ContentType::BuildOutput => 2048,
            ContentType::Error => 1536,
            ContentType::Json => 4096,
            ContentType::Terminal => 1024,
            ContentType::Git => 2048,
            ContentType::Text => 4096,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ContentType::Code => "code",
            ContentType::Diff => "diff",
            ContentType::BuildOutput => "build_output",
            ContentType::Error => "error",
            ContentType::Json => "json",
            ContentType::Terminal => "terminal",
            ContentType::Git => "git",
            ContentType::Text => "text",
        }
    }
}

pub struct Classifier {
    /// Overrides applied on top of the defaults (hot-reloadable).
    overrides: HashMap<ContentType, usize>,
    /// Skip CCR for outputs that look "clean" (no markers of heavy content).
    poll_filter: bool,
}

impl Classifier {
    pub fn new(overrides: HashMap<ContentType, usize>, poll_filter: bool) -> Self {
        Classifier { overrides, poll_filter }
    }

    /// Score-based type detection. Each rule adds weight; the type with the
    /// highest score wins. Ties resolve to Text.
    pub fn classify(&self, output: &str) -> ContentType {
        let mut scores: HashMap<ContentType, usize> = HashMap::new();

        for line in output.lines().take(200) {
            let line = line.trim_start();
            if line.is_empty() {
                continue;
            }

            // Code: leading `//`, `use `, `fn `, `pub `, braces, keywords.
            if line.starts_with("//")
                || line.starts_with("use ")
                || line.starts_with("fn ")
                || line.starts_with("pub ")
                || line.starts_with("impl ")
                || line.starts_with("struct ")
                || line.starts_with("enum ")
            {
                *scores.entry(ContentType::Code).or_insert(0) += 1;
            }

            // Diff: unified diff headers and hunks.
            if line.starts_with("diff --git")
                || line.starts_with("+++ ")
                || line.starts_with("--- ")
                || line.starts_with("@@ ")
            {
                *scores.entry(ContentType::Diff).or_insert(0) += 2;
            }

            // Build output: cargo/rustc markers.
            if line.starts_with("Compiling ")
                || line.starts_with("Checking ")
                || line.starts_with("Finished ")
                || line.starts_with("error[E")
                || line.starts_with("warning: unused")
            {
                *scores.entry(ContentType::BuildOutput).or_insert(0) += 1;
            }

            // Errors: rustc/panic/backtrace shapes.
            if line.starts_with("error[")
                || line.starts_with("panicked at ")
                || line.starts_with("thread 'main' panicked")
            {
                *scores.entry(ContentType::Error).or_insert(0) += 3;
            }

            // JSON: object/array starters, quote-heavy lines.
            if line.starts_with('{') || line.starts_with('[') || line.starts_with('"') {
                *scores.entry(ContentType::Json).or_insert(0) += 1;
            }

            // Git: porcelain/status/log shapes.
            if line.starts_with("On branch ")
                || line.starts_with("nothing to commit")
                || line.starts_with("Switched to branch")
                || line.starts_with(" * [new tag]")
                || line.starts_with("EXIT:")
            {
                *scores.entry(ContentType::Git).or_insert(0) += 1;
            }

            // Terminal: shell prompt / command echo shapes.
            if line.starts_with("$ ") || line.starts_with("~/") || line.starts_with("EXIT:$?") {
                *scores.entry(ContentType::Terminal).or_insert(0) += 1;
            }
        }

        if self.poll_filter && scores.is_empty() {
            return ContentType::Text; // clean output - skip CCR
        }

        scores
            .into_iter()
            .max_by_key(|(_, v)| *v)
            .map(|(ty, _)| ty)
            .unwrap_or(ContentType::Text)
    }

    /// Threshold check: does this output qualify for CCR compression?
    pub fn should_compress(&self, ty: ContentType, size: usize) -> bool {
        let threshold = self.overrides.get(&ty).copied().unwrap_or_else(|| ty.default_threshold());
        size >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_code() {
        let c = Classifier::new(HashMap::new(), true);
        assert_eq!(c.classify("// header\npub fn main() {}"), ContentType::Code);
    }

    #[test]
    fn classifies_diff() {
        let c = Classifier::new(HashMap::new(), true);
        let d = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,3 +1,4 @@\n+let x = 1;";
        assert_eq!(c.classify(d), ContentType::Diff);
    }

    #[test]
    fn threshold_boundary() {
        let c = Classifier::new(HashMap::new(), true);
        assert!(!c.should_compress(ContentType::Code, 4095));
        assert!(c.should_compress(ContentType::Code, 4096));
        assert!(c.should_compress(ContentType::Diff, 3072));
        assert!(!c.should_compress(ContentType::Terminal, 1023));
    }
}
""",
        ),
        # Turn 9: Assistant answers from the single retrieved entry
        Turn(
            role="assistant",
            content="""Retrieved classifier.rs only (5.2 KB). From it:

**Classification:** `classify()` scores each line by shape - leading tokens (`//` → code, `diff --git` → diff, `error[E...]` → error, `{`-heavy → json, `Compiling`/`Finished` → build_output, `On branch`/`EXIT:` → git) and picks the highest-scoring ContentType; ties and clean outputs fall back to Text (the poll pre-filter).

**Threshold boundary:** an output is compressed only when `size >= threshold[type]` - code/json 4096, diff 3072, build/git 2048, error 1536, terminal 1024. So a 3 KB diff gets compressed; a 3 KB code block does not. Overrides come from the hot-reloadable config map.

**Preview:** the first `preview_max_chars` (default 400) chars plus a structure map of function/struct names. The preview is what a marker expands to in prompts - the full body stays in CCR until `aphrodite_retrieve`.

Not expanded: engine.rs (12.4 KB), store.rs (8.9 KB), marker.rs (3.2 KB) - all still markers. One entry retrieved out of four.""",
        ),
        # Turn 10: User asks for the token-saving summary
        Turn(
            role="user",
            content="That answers it. Now summarize the token-saving decisions you made this session.",
        ),
        # Turn 11: Final summary of context-minimization choices
        Turn(
            role="assistant",
            content="""Token-frugal decisions, in order:

1. **Prefetch, not read** - 4 files (~29.9 KB raw) became 4 CCR markers + previews (~600 B in context). Raw content never entered context.
2. **Search with previews** - `aphrodite_search` returned 6 compact hits with line numbers instead of grepping full files.
3. **Compact catalog + stats** - confirmed what was storable and how much was saved without expanding anything.
4. **Retrieve-only-when-needed** - exactly one `aphrodite_retrieve` (classifier.rs, the entry the question needed); the other 3 markers stayed unexpanded.
5. **Preview-over-raw reporting** - answered from the preview plus the single retrieved body; quoted only the needed lines.

Net effect: ~29.9 KB of source stayed out of context - roughly 7000 tokens saved versus reading the files raw. This is the behavior the compression proxy is designed to enable: markers replace bulk, retrieval is opt-in per question.""",
        ),
    ],
)
