//! code shape detector.

use crate::preview::{
	input::Input,
	line::code::{is_code_strong_line, is_code_vote_line},
};

/// A strong signature marker (`fn main(`, `def x(`, `struct X`, `#include`,
/// shebang) or >=2 statement-line votes (`use x::y;`, `let x =`, `import x`,
/// `return x`, ...).
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let code_strong = inp.non_empty.iter().any(|l| is_code_strong_line(l.trim_start()));
	let code_votes = inp.non_empty.iter().filter(|l| is_code_vote_line(l.trim_start())).count();
	code_strong || code_votes >= 2
}
