//! Detection entry points.
//!
//! Two questions are asked of content before it is previewed: what does the
//! vendored classifier think it is, and does Aphrodite recognise a shape the
//! classifier flattened. They are separate calls because they have separate
//! authority - the classifier is a first opinion, the semantic detector is an
//! override.

pub mod SemanticType;
pub mod Type;
