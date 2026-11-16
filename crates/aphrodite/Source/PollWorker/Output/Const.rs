//! The hard ceiling on accumulated poll output.

/// Maximum bytes of poll output kept per task (256 KB).
///
/// Enforced by `Output::Append`, not by a length test at the call site: a test
/// taken before an append cannot bound an append whose size it never looked at,
/// which is how the accumulator used to exceed its own stated cap by one whole
/// poll payload.
pub(crate) const Maximum:usize = 256 * 1024;
