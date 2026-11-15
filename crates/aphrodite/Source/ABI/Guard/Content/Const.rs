//! The largest payload any single export accepts.

/// Largest payload any single export accepts, in bytes (16 MB).
///
/// A C ABI caller hands over an already-materialised string, so an unbounded
/// length is an out-of-memory request rather than a slow path. The compression
/// and hook entry points (`AphroditeCompress`, `AphroditeTransform`,
/// `AphroditeTerminal`, `AphroditeDispatch`) and `AphroditeClassify` reject
/// anything larger with an error payload instead of attempting the copy; the
/// stateless text utilities (`FilterLines`, `Preview`, `stage2`,
/// `struct_extract`) do not check it, so the cap is a guard on the paths that
/// store, not a universal limit.
pub(crate) const MaximumContent:usize = 16 * 1024 * 1024; // 16MB cap
