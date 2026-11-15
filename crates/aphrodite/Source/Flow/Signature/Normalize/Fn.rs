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
pub fn NormalizeArgsSig(Tool:&str, Args:Option<&serde_json::Value>) -> u64 {
	let mut Buffer = String::new();
	Buffer.push_str(Tool);
	Buffer.push('\u{1}');
	if let Some(v) = Args {
		// Terminal-style calls: the command string is the whole identity.
		if let Some(Command) = v.get("command").and_then(|c| c.as_str()) {
			Buffer.push_str(Command.trim_end());
		} else if let Some(Obj) = v.as_object() {
			let mut Keys:Vec<&String> = Obj.keys().filter(|k| !VolatileKeys.contains(&k.as_str())).collect();
			Keys.sort();
			for k in Keys {
				Buffer.push_str(k);
				Buffer.push('=');
				// Compact, stable string form of the value.
				Buffer.push_str(&Obj[k].to_string());
				Buffer.push('\u{1f}');
			}
		} else {
			Buffer.push_str(&v.to_string());
		}
	}
	Fnv1a(Buffer.as_bytes())
}
