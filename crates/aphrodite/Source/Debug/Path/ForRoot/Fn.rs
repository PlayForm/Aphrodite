//! `ForRoot`: the log file for a named session.

use std::path::PathBuf;

use crate::Debug::Path::Runtime::Fn::Fn as Runtime;

/// DEV-ONLY: the log file path for a root session (test/visibility helper -
/// honors the test-home isolation).
///
/// Takes the root as an argument instead of reading the current one so the
/// bridge can resolve the path of a session it is not hosting, and so a test
/// never has to publish a global to ask where a file would land.
pub fn Fn(Root:&str) -> PathBuf { Runtime().join(format!("debug.{Root}")) }
