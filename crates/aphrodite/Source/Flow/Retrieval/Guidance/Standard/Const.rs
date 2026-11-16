//! The `standard` retrieval-guidance line.

/// Retrieval-guidance text for `retrieve_guidance = "standard"`.
///
/// Wire contract: this is model-visible prompt text, so the sentence is the
/// contract. It teaches retrieval vocabulary only - never how the compression
/// pipeline works.
pub(crate) const Text:&str = "CCR markers compress large tool output. Use aphrodite_retrieve(hash) when the preview \
                              doesn't tell you enough; batch pending markers into one turn.";
