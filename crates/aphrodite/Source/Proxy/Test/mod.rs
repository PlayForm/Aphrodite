//! Proxy regression tests - the behavioural contract of the module tree.
//!
//! Split by the area each test pins, one file per area, with the shared
//! fixtures in `Support`. The bodies are verbatim moves: a green run before
//! and after is the proof that only the layout changed. A broken or
//! misleading preview is a bug, so no test here may be weakened to fit a
//! refactor.

pub mod Body;
pub mod CCR;
pub mod Cache;
pub mod Compress;
pub mod Detect;
pub mod Metadata;
pub mod Preview;
pub mod Property;
pub mod Regression;
pub mod Relay;
pub mod Stats;
pub mod Stream;
pub mod Threshold;
pub mod Support;

// `pub(crate)` (not private) so other in-crate test modules - e.g.
// `Retrieve::tests` - can reach `test_state_with_ccr()` without duplicating
// `AppState`'s ~47-field literal (report 05 T5 verification).
