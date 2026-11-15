//! Cap on the combined injected directive text.

/// Cap on the combined injected text across all active directives (01-F5).
///
/// The per-file cap alone does not bound this: with several directives active at once,
/// each already-capped body still stacks up in the context string. Measured in
/// CHARACTERS - `Out.len()` is a byte length, and using it tripped the truncation on
/// multibyte content (em-dashes) that never exceeded the char budget, silently cutting
/// directive text that fit (T19).
pub(crate) const Const:usize = 4000;
