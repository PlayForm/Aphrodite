//! `/metrics` - the Prometheus text exposition, as one ordered table.
//!
//! The document is a table of `Piece` values: each piece is either one counter
//! `Entry` (a name, a path into the stats document, and the shape of the number)
//! or the latency histogram. `Body` renders the table in order, so the set of
//! metrics this endpoint exposes is one list rather than a sequence of branches.
//!
//! Nothing here branches on a value: an absent member of the stats document
//! contributes no line, which is what the `Option` in `Entry::Line` carries.

pub mod Body;
pub mod Entry;
pub mod Histogram;
pub mod Kind;
pub mod Label;
pub mod Node;
pub mod Piece;
pub mod Route;
pub mod Table;
