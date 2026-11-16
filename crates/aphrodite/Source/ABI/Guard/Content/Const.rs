//! The largest payload any single export accepts.

/// Largest payload any single export accepts, in bytes (16 MB).
///
/// A C ABI caller hands over an already-materialised string, so an unbounded
/// length is an out-of-memory request rather than a slow path.
///
/// The number is now enforced at the INGRESS rather than at acceptance:
/// `Guard::Cstr` refuses any pointer argument longer than this before it copies
/// it, and `Route::Decode` refuses a JSON payload longer than this before it
/// reaches a command. That makes it a bound on every export, including the
/// stateless text utilities (`FilterLines`, `Preview`, `stage2`,
/// `struct_extract`), which used to accept any size - they still scanned and
/// copied the whole string, so the only thing they gained was the allocation.
/// The per-export checks that remain (`Compress`, `Transform`, `Terminal`,
/// `Classify`) are the same rule stated by the export that stores, kept because
/// the store is what the cap exists to protect.
pub(crate) const Maximum:usize = 16 * 1024 * 1024; // 16MB cap
