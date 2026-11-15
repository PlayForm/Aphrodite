//! The process-global template registry.
//!
//! Three leaves, one singleton: the state struct (`Struct/`), the cell that
//! holds it for the process (`Const/`), and the accessor every reader and
//! writer goes through (`Fn/`). Splitting them means a new reader cannot
//! accidentally introduce a second `OnceLock` and a second registry.

pub mod Const;
pub mod Fn;
pub mod Struct;
