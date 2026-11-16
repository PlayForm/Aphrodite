//! Argument signatures: the same logical call must hash the same twice.

use crate::Flow::Signature::Normalize::Fn::Fn as Signature;

#[test]
fn Volatile() {
	let a = serde_json::json!({"path": "src/x.rs", "timeout": 30, "session_id": "abc"});
	let b = serde_json::json!({"path": "src/x.rs", "timeout": 99, "session_id": "zzz"});
	assert_eq!(
		Signature("read_file", Some(&a)),
		Signature("read_file", Some(&b)),
		"volatile keys must not affect the signature"
	);
	// A different file path yields a different signature.
	let c = serde_json::json!({"path": "src/y.rs"});
	assert_ne!(Signature("read_file", Some(&a)), Signature("read_file", Some(&c)));
	// Same command twice => same sig (terminal path).
	let Cmd1 = serde_json::json!({"command": "cargo test   "});
	let Cmd2 = serde_json::json!({"command": "cargo test"});
	assert_eq!(Signature("terminal", Some(&Cmd1)), Signature("terminal", Some(&Cmd2)));
}
