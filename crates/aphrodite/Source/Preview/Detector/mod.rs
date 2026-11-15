//! Shape detectors, one file per content shape.
//!
//! A detector answers one question - does this content have this shape - and the
//! order of the chain in `Detect::SemanticType` is the priority contract. Keeping
//! each shape in its own file is what lets that order be read as a list. A
//! detector is not a builder: it never renders, and it never sees the preview.

pub mod Build;
pub mod Code;
pub mod CSV;
pub mod Diff;
pub mod Git;
pub mod GitLog;
pub mod Grep;
pub mod JSON;
pub mod Log;
pub mod Ls;
pub mod Markdown;
pub mod Search;
pub mod Table;
pub mod Terminal;
pub mod Test;
pub mod XML;
pub mod YAML;
