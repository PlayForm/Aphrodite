//! Where the debug log lives.
//!
//! Three answers: the directory the log is written into (`Runtime/`), the file
//! for the current session (`Log/`), and the file for a named session
//! (`ForRoot/`). The directory is the only place the test-home override is
//! honoured, so every path question above it inherits the isolation.

pub mod ForRoot;
pub mod Log;
pub mod Runtime;
