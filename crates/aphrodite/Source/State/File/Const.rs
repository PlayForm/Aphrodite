//! The referenced-files list's bound.

/// Distinct paths the referenced-files list may hold before the
/// least-recently-referenced one is dropped.
///
/// The list is re-ordered on every reference (the touched path moves to the
/// front), so the truncation at this length always drops the end that has gone
/// longest without being touched.
pub(crate) const Cap:usize = 100;
