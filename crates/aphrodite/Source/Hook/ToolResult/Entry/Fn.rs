//! The tool-result entry points.
//!
//! Three signatures, one pipeline: the plain form, the form that accepts a
//! caller-supplied classification, and the form that also carries Hermes' call
//! metadata. They exist as separate functions because the FFI, the proxy and
//! the bridge each know a different amount about the call; they must never
//! fork on what the pipeline does with it.

use super::super::Inner::Fn::Fn as TransformToolResultInner;
use crate::Hook::Meta::Struct::Struct as ToolCallMeta;
use crate::State::Struct::AphroditeState;

/// Transform tool output - full compression pipeline.
pub fn TransformToolResult(State:&mut AphroditeState, Content:&str, ToolName:&str) -> serde_json::Value {
	TransformToolResultInner(State, Content, ToolName, None, &ToolCallMeta::default())
}

/// Same as [`TransformToolResult`], but a caller that has already found the
/// "real" payload underneath a wrapper the core classifier can't see through
/// (e.g. an agent-specific JSON envelope) may supply it via `Classify` as
/// `(content_to_classify, type)`. Core stays agnostic to what a wrapper looks
/// like - it only ever hashes and stores the ORIGINAL `Content`, so
/// `AphroditeRetrieve` always returns exactly what was passed in; `Classify`
/// affects only the reported `type` and the generated preview.
pub fn TransformToolResultClassified(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
) -> serde_json::Value {
	TransformToolResultInner(State, Content, ToolName, Classify, &ToolCallMeta::default())
}

/// Same as [`TransformToolResultClassified`], plus the Hermes-supplied call
/// metadata (report 05, P2/T6) that gets recorded into the `ToolEvents`
/// telemetry ring. This is the entry point the Hermes bridge routes to so the
/// error/args/duration signals Hermes ships on every call stop being dropped.
pub fn TransformToolResultWithMeta(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
	meta:&ToolCallMeta,
) -> serde_json::Value {
	TransformToolResultInner(State, Content, ToolName, Classify, meta)
}
