//! `Database`: the token-mode SQLite CCR database.

use std::path::PathBuf;

use crate::Home::Runtime::Fn::Fn as RuntimeHome;

/// `ccr.db` under the runtime home (token-mode SQLite CCR database).
///
/// The default only: `[proxy] ccr_db_path` / `$APHRODITE_DB` / `--ccr-db`
/// override it further up the stack, and a relative override is resolved
/// against the binary directory rather than the cwd. Storing this path here
/// keeps the DEFAULT in one place; it is not the final answer.
pub fn Fn() -> PathBuf { RuntimeHome().join("ccr.db") }
