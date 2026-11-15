//! The marker grammar - reading `<<<CCR:hash|type|size>>>` back apart.
//!
//! The grammar is a wire format shared with the model: the marker builder writes
//! it, the resolver reads it, and both sides must agree byte for byte. So the
//! delimiters live in exactly two files, hash extraction lives in one, and the
//! scanner that finds markers in arbitrary text lives in one. It is not a
//! tokenizer and it does not validate hashes - `Marker::Parse` owns validity;
//! this module only finds the shape and pulls the first field out of it.
//!
//! Nothing here is public API: the delimiters and the two readers are crate
//! internals, and a caller outside the resolver has no business assuming the
//! grammar's spelling.

pub mod Find;
pub mod Hash;
pub mod Prefix;
pub mod Suffix;
