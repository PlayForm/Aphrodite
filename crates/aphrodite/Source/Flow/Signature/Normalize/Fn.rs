//! FNV-1a signature of a tool call plus its normalized arguments.
//!
//! Two callers rely on this being stable across runs: the turn telemetry
//! (repeat detection) and the adaptive chain-split threshold (P2/P8 similarity
//! key). A signature that drifts makes the same command look new every turn,
//! which inflates the split decision until it misfires.

use crate::Flow::{Hash::Fn::Fnv1a, Signature::Const::VolatileKeys};

/// FNV-1a signature of `Tool` + normalized args (P2/P8 similarity key).
///
/// Args are normalized by serializing the object with keys sorted and volatile
/// keys stripped, so the same logical call hashes stably; a different file path
/// yields a different signature. Terminal calls (args carrying a `command`
/// string) normalize to the trimmed command string.
pub fn Fn(Tool:&str, Args:Option<&serde_json::Value>) -> u64 {
	let mut Buffer = String::new();
	Buffer.push_str(Tool);
	Buffer.push('\u{1}');

	// The arguments are the value that may be absent: when the call carried none
	// there is nothing to normalize, and `for_each` is the one-item iteration over
	// the `Option` rather than a guard in front of the work.
	Args.into_iter().for_each(|v| {
		// Terminal-style calls: the command string is the whole identity.
		match v.get("command").and_then(|c| c.as_str()) {
			Some(Command) => {
				Buffer.push_str(Command.trim_end());
			},
			None => {
				match v.as_object() {
					Some(Obj) => {
						let mut Keys:Vec<&String> =
							Obj.keys().filter(|k| !VolatileKeys.contains(&k.as_str())).collect();
						Keys.sort();

						Keys.into_iter().for_each(|k| {
							Buffer.push_str(k);
							Buffer.push('=');
							// Compact, stable string form of the value.
							Buffer.push_str(&Obj[k].to_string());
							Buffer.push('\u{1f}');
						});
					},
					None => {
						Buffer.push_str(&v.to_string());
					},
				}
			},
		}
	});

	Fnv1a(Buffer.as_bytes())
}
