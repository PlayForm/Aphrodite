//! True when a field value is prose rather than a re-embedded payload.

/// True when `Text` is non-empty prose rather than an embedded JSON payload.
///
/// An empty string carries no content, and a leading `{`/`[` means the field
/// restates a structure the caller already has - neither is a payload worth
/// surfacing, so both fall through to the next matcher (or to the caller's own
/// classification of the raw content).
pub(crate) fn Fn(Text:&str) -> bool { !Text.is_empty() && !Text.starts_with('{') && !Text.starts_with('[') }
