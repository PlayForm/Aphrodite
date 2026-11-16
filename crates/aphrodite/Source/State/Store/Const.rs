//! The inline store's byte budget.

/// Default byte budget for the inline store (report 05 F11).
///
/// The entry-count cap alone (`Entry::Const::Cap`) doesn't bound memory -
/// `aphrodite_prefetch` admits files up to 10MB each and the ABI admits blobs up
/// to 16MB, so 500 entries at the large end is a multi-GB worst case with zero
/// byte accounting. 256MB is a conservative default for a single agent
/// session's compression cache; exposed via `Store::Struct::Set` so a config
/// layer can override it.
pub const Budget:usize = 256 * 1024 * 1024;
