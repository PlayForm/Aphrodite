//! `[prompts]` - how the system instructs the LLM about CCR.

/// The `[prompts]` table.
///
/// These knobs teach vocabulary, never mechanism: how much retrieval guidance
/// to inject, whether markers get a hint, whether catalog rows are annotated.
/// They cannot change what is compressed - only how the model is told to talk
/// about it.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Struct {
	/// Retrieval guidance level (`minimal`/`standard`/`verbose`).
	pub RetrieveGuidance:Option<String>,
	/// Show the marker hint line alongside compressed output.
	pub CCRMarkerHint:Option<bool>,
	/// Annotate recall-catalog rows with `intent=read`.
	pub CatalogIntentHints:Option<bool>,
}
