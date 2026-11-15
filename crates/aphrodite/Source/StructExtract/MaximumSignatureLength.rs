//! The cap on one signature line.
//!
//! Named `Const` because the file name carries the domain, per the house style.
#![allow(non_upper_case_globals)]

/// Characters one rendered signature line may occupy: 60.
///
/// `Format::Signature::Fn` measures the whole line - kind, name, parameter list
/// and return type - against this, not the name alone, and cuts to `Const - 3`
/// bytes so the three dots of the ellipsis fit inside the cap. It is not the
/// preview budget: `Budget::Const` caps the sum of all lines, this caps each one.
/// A line that overruns is cut mid-parameter-list, so the model still sees the
/// arity but loses the tail of the parameter names.
pub(crate) const Const:usize = 60;
