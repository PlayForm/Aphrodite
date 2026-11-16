//! Request and response body plumbing.
//!
//! The body is where a proxy silently breaks: a cap enforced after the read has
//! already buffered the memory it was meant to save, a stream classified as a
//! buffer, a multibyte char split at a byte budget, a preview copied into an
//! owned `String` for a formatter that takes a `&str`. Each concern gets its own
//! leaf so none of them is edited by accident while touching another.
//!
//! The request body's delivery mode is not decided here - it is read off the
//! single request interpretation (`Request::Semantics`), because a second
//! predicate over the same bytes is how one body came to be classified two ways.

pub mod Capped;
pub mod Chars;
pub mod TooLarge;
pub mod Truncate;
