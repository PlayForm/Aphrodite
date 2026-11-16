//! The separator between two members of a canonical object or array.

/// The bytes that precede the member at `Index`: nothing for the first, a comma
/// for every one after it.
///
/// A separator is a function of the position, not a branch inside the walk, so the
/// canonical form has one place where the punctuation is decided.
pub fn Fn(Index:usize) -> &'static [u8] { (Index > 0).then_some(&b","[..]).unwrap_or(&b""[..]) }
