//! The two width caps on a rendered signature.
//!
//! A signature is capped twice: the parameter list inside the parentheses, and the
//! whole line once it is joined. Both are `Const` leaves because both are one number
//! that a caller must not re-spell - the directory names which cap is which, so the
//! leaf only has to say "constant".

pub mod Parameters;
pub mod Signature;
