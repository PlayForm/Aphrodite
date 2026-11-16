//! The marker grammar - reading `<<<CCR:hash|type|size>>>` back apart.
//!
//! The grammar is a wire format shared with the model: the marker builder writes it, the
//! resolver reads it, and both sides must agree byte for byte. So the delimiters live in
//! exactly two files, hash extraction lives in one, the scanner that finds markers in
//! arbitrary text lives in one, and what a marker-shaped region that is NOT a usable
//! reference means lives in [`Tolerance`] - one policy, one place.
//!
//! An occurrence is an [`Occurrence::Struct::Struct`]: a span, the marker text and the hash,
//! all borrowed from the document they were found in. That is what lets the walk build its
//! output in one pass instead of re-searching text it has already written.
//!
//! It is not a tokenizer and it does not validate hashes - `Marker::Parse` owns validity;
//! this module finds the shape and pulls the first field out of it. Nothing here is public
//! API: the delimiters and the readers are crate internals, and a caller outside the resolver
//! has no business assuming the grammar's spelling.

pub mod Find;
pub mod Hash;
pub mod Occurrence;
pub mod Prefix;
pub mod Suffix;
pub mod Tolerance;
