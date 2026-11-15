//! Headroom classifier wrapper.
//!
//! The vendored Headroom classifier is the broad first opinion about what a blob
//! is. Downstream crates need that opinion without taking a direct `headroom-core`
//! dependency, so it is re-exported through this one call. It is not the semantic
//! detector: this is the classifier's verdict alone, and `SemanticType` is what
//! overrides it.

use headroom_core::transforms;

/// Detect the CCR content-type string for a blob (e.g. `source_code`, `Build`,
/// `json_array`).
///
/// A thin wrapper over the Headroom classifier so `aphrodite-hermes` does not
/// need a direct `headroom-core` dependency. It is not a semantic judgement: the
/// classifier flattens several common tool-output shapes to bare `Text` or
/// `terminal`, and `SemanticType::Fn` is the layer that recovers them.
pub fn Fn(Content:&str) -> String {
	transforms::content_detector::detect_content_type(Content)
		.content_type
		.as_str()
		.to_string()
}
