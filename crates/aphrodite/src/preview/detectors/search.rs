//! search shape detector (Phase 6: `path:line:` majority, mirroring the old
//! proxy `SEARCH_RESULT_PATTERN` shape).

use crate::preview::input::Input;
use crate::preview::line::grep::is_search_line;

/// `path:line:` majority - the same structural rule the search preview uses
/// (`is_search_line`: `splitn(3, ':')`, path non-empty no-space, middle all
/// digits, third segment exists).
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	inp.majority(is_search_line, 2)
}