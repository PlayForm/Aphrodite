//! `aphrodite_test` - the in-process compress/retrieve smoke test.

use crate::{
	Health::Proxy::Fn::proxy_health,
	Tool::{Arg, Compress, Dispatch::Fn::ToolHandler},
};

/// The `aphrodite_test` handler.
///
/// The check is a real round trip through the session store, not a comparison of
/// the returned hash: it re-expands the marker and compares bytes, which is the
/// only assertion that catches a store/preview mismatch.
///
/// It is not hermetic - the samples it compresses stay in the caller's session, so
/// a test call adds catalogue rows. That is deliberate: it exercises the same path
/// a real compression takes.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_test", |args| {
		let mode = {
			let m = Arg::Fn::Fn(args, "mode");

			match m.is_empty() {
				true => "quick",
				false => m,
			}
		};
		let samples:&[(&str, &str)] = match mode {
			"quick" => &[("fn main() { println!(\"hi\"); }\n", "source_code")],
			_ => {
				&[
					("fn main() { println!(\"hi\"); }\n", "source_code"),
					("error[E0382]: borrow of moved value\nwarning: unused\n", "build"),
					("{\"a\":1,\"b\":2,\"c\":3}\n", "json_array"),
				]
			},
		};
		let mut checks = Vec::new();
		let mut passed = 0usize;
		crate::Session::With::Fn::with_shared(|state| {
			samples.iter().for_each(|(content, hint)| {
				let info = Compress::Fn::Fn(state, content, hint, None);
				let hash = info["hash"].as_str().unwrap_or("");
				let round = aphrodite::Resolve::Expand::Fn(state, hash);
				let ok = round.as_deref() == Some(*content);

				// A round trip that matched is one more pass: the count is a
				// value, not a branch.
				passed += usize::from(ok);

				checks.push(serde_json::json!({
					"type": hint, "hash": hash,
					"roundtrip": ok,
				}));
			});
		});

		let status = match passed == checks.len() {
			true => "ok",
			false => "fail",
		};

		serde_json::json!({
			"mode": mode,
			"status": status,
			"passed": passed,
			"total": checks.len(),
			"checks": checks,
			"proxies": proxy_health(),
		})
	})
}
