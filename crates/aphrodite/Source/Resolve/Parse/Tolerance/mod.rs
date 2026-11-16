//! The marker grammar's tolerance policy, in one place.
//!
//! The grammar is a wire format the model also writes, so a document can contain
//! marker-shaped text that is not a usable reference. There are exactly four such cases,
//! each with one answer, and the answers are a POLICY rather than an accident of the
//! scanner - which is why they are stated here and implemented by one leaf each:
//!
//! - **Unterminated** - an opening delimiter with no closing one. The scan steps over
//!   the delimiter and keeps looking: the writer never finished emitting it, so guessing
//!   at its extent would resolve text nobody wrote.
//! - **Hashless** - a complete marker naming no hash. It is not an occurrence at all;
//!   the scan continues after it.
//! - **Reference** - a complete marker naming a hash. Emitted and resolved.
//! - **Unresolved** - a reference whose hash the store does not hold. Its ORIGINAL
//!   marker text stays in place, because a merely evicted or late entry can still heal
//!   (F1). This one is not a grammar question - it is answered by
//!   [`crate::Resolve::Nested::Enum::Enum::Unresolved`], at the one site that resolves an
//!   occurrence.
//!
//! The first three are [`Enum`] and [`Fn`], which the scanner asks once per candidate
//! region. `Resolve::Test::Tolerance` is the same table as executable cases, so a change
//! to any row has to be made in both places deliberately.

pub mod Enum;
pub mod Fn;
