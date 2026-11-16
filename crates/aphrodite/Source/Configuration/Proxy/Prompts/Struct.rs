//! `[prompts]` - how the system instructs the LLM about CCR.

/// The `[prompts]` table.
///
/// These knobs teach vocabulary, never mechanism: how much retrieval guidance
/// to inject, whether markers get a hint, whether catalog rows are annotated.
/// They cannot change what is compressed - only how the model is told to talk
/// about it.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// `[prompts] retrieve_guidance`; how much retrieval explanation is
	/// injected per turn (`minimal`/`standard`/`verbose`). `None` = `minimal`.
	#[serde(rename = "retrieve_guidance")]
	pub Guidance:Option<String>,
	/// `[prompts] ccr_marker_hint`; show the marker hint line alongside
	/// compressed output. `None` = no hint line.
	#[serde(rename = "ccr_marker_hint")]
	pub Hint:Option<bool>,
	/// `[prompts] catalog_intent_hints`; annotate recall-catalog rows with
	/// `intent=read`. `None` = no annotation.
	#[serde(rename = "catalog_intent_hints")]
	pub Intent:Option<bool>,
}
