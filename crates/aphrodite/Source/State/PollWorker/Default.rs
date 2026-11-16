//! The poll-worker seed: no background tasks, auto-backgrounding on.

use std::collections::VecDeque;

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self { Self { Tasks:VecDeque::new(), Enable:true } }
}
