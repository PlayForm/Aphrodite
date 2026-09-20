use std::collections::HashMap;

use crate::state::AphroditeState;
use super::recursive::resolve_recursive;

/// Public entry point: expands a hash through all nesting levels.
pub fn expand(state:&mut AphroditeState, hash_val:&str) -> Option<String> {
	let mut resolved = HashMap::new();
	let mut visited = Vec::new();
	resolve_recursive(state, hash_val, 0, &mut resolved, &mut visited)
}
