//! `aphrodite_test` - the in-process compress/retrieve smoke test.

use crate::{
	Tool::{Arg, Compress, Dispatch::ToolHandler},
	proxy_health,
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
			let m = Arg::Fn(args, "mode");
			if m.is_empty() { "quick" } else { m }
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
		crate::with_shared(|state| {
			for (content, hint) in samples {
				let info = Compress::Fn(state, content, hint, None);
				let hash = info["hash"].as_str().unwrap_or("");
				let round = aphrodite::Resolve::Expand::Fn(state, hash);
				let ok = round.as_deref() == Some(*content);
				if ok {
					passed += 1;
				}
				checks.push(serde_json::json!({
					"type": hint, "hash": hash,
					"roundtrip": ok,
				}));
			}
		});
		serde_json::json!({
			"mode": mode,
			"status": if passed == checks.len() { "ok" } else { "fail" },
			"passed": passed,
			"total": checks.len(),
			"checks": checks,
			"proxies": proxy_health(),
		})
	})
}
