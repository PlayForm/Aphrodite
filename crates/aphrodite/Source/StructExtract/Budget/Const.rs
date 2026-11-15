//! The total character budget for one structure preview.

/// Characters of signature text one structure preview may carry: 300.
///
/// The budget is shared across every category a language emits - Rust pays for `Fns`, then
/// `Structs`, then `Traits`, then `Impls` out of the same counter - so it is not a per-category cap
/// and it is not advisory. Each admitted signature subtracts its length plus its one-character
/// separator and the scan stops as soon as the counter reaches zero, which is why the tail categories
/// of a long file are dropped rather than each category being cut evenly. Raising it inflates every
/// code marker the model reads; lowering it loses structure the model would otherwise have been
/// given, and it loses it silently.
pub(crate) const Const:usize = 300;
