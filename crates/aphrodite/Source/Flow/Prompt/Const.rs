//! The compiled-in prompt strings.
//!
//! Every entry here is a FALLBACK, never an override: a `[templates.prompts]`
//! key in aphrodite.toml wins whenever it is present, so editing a string here
//! changes what the model reads only for configurations that leave that key
//! out. The shipped text has to stand on its own for a user who never opens
//! the TOML, and it must never quietly shadow a configured one.

/// Compiled-in fallback prompt templates - the `[templates.prompts]` keys
/// (pre-Rust port of `_core/template.py` `_render_prompt_tmpl` fallbacks),
/// with the Rust-side current strings where those already ship compiled in
/// (`SessionInject`, `search_hint`). A key absent from the TOML map falls
/// back to its entry here; an unknown key renders as "" (never panics).
pub const Fallback:&[(&str, &str)] = &[
	("session_inject", Shipped),
	(
		"engine_offload",
		"These messages were offloaded to reduce context. Use aphrodite_retrieve({hash}) if needed. The {tail} \
		 messages below are your active context.",
	),
	(
		"auto_expand_guidance",
		"Tool outputs are auto-expanded - you see full content inline. If you see a CCR marker, retrieve only if the \
		 preview hints at useful content.",
	),
	(
		"catalog_context_warn",
		"context={ctx} msgs - prefer catalog over scanning history",
	),
	("search_hint", "Use aphrodite_retrieve(hash) to expand any entry."),
];

/// Default first-turn session injection. Loaded from `[prompts] session_inject`
/// in aphrodite.toml; this constant is the fallback when the key is absent.
/// Rendered once (turn_counter == 0) by `Flow::FirstTurn::Fn::Fn`, then
/// dropped. Empty string disables it entirely.
pub const Shipped:&str = "\
[APHRODITE] v{VERSION} active.
  This session is running with CCR compression. Tool outputs larger than a
  few hundred bytes are replaced with markers like <<<CCR:hash|type|size>>>.
  The marker is a handle to the content: aphrodite_retrieve(hash) returns
  the full original (sub-ms, local). Read the marker's type and size to
  decide whether the full content is needed; retrieve when your next action
  requires it, skip when the preview or type/size already answers. You can
  batch pending markers and retrieve selectively.
  Never re-read a file you already have a marker for. Use aphrodite_catalog
  to see stored entries, aphrodite_search to locate content by keyword, and
  aphrodite_prefetch for background file loads. Prefer granular retrieval
  over wholesale: pull the exact piece you need.
  Layer 2: per-turn catalog injected below each turn.
  Layer 3: load the aphrodite-tool-guide skill for full tool reference.";
