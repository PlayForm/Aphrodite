//! Where a finished task's output was stored.

pub mod Struct;

/// The test accessor for a status's stored output. The production paths match the
/// lifecycle variant directly, so this exists for tests only and is compiled with them.
#[cfg(test)]
pub mod Fn;
