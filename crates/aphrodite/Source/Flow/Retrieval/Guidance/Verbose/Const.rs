//! The `verbose` retrieval-guidance line.

/// Retrieval-guidance text for `retrieve_guidance = "verbose"`.
///
/// Wire contract: model-visible prompt text - the sentence is the contract.
/// Same vocabulary as `Standard`, plus the granularity and no-re-read rules.
pub(crate) const Text:&str = "CCR markers compress large tool output. Use aphrodite_retrieve(hash) when the preview \
                              doesn't tell you enough; batch pending markers into one turn. Prefer granular retrieval \
                              - pull the exact piece you need. Never re-read a file you hold a marker for; never \
                              invent content you did not retrieve.";
