//! Line-window pagination of a resolved payload.
//!
//! One request, four questions, one leaf each: `Mode/` is what was asked for
//! (`Exact` or a `Window`), `Slice/` cuts the document, `Outcome/` is what
//! cutting produced, and `Render/` turns that outcome into the wire body and the
//! truncation flag. `Fn/` is the compatibility adapter for the old
//! `(offset, limit)` pair and holds no arithmetic of its own.
//!
//! The split exists because the two requests are two contracts: `Exact` must
//! return the original bytes so the body hashes to its own marker hash, while a
//! `Window` must return an annotated range. `Limit == 0` used to mean both.

pub mod Fn;
pub mod Mode;
pub mod Outcome;
pub mod Render;
pub mod Slice;
