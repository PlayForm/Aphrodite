//! `aphrodite_test` - the in-process compress/retrieve smoke test.

use crate::{
	Health::Proxy::Fn::proxy_health,
	Tool::{Compress, Dispatch::Fn::Handler, Registry::Test::Mode},
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
pub(crate) fn Fn() -> (&'static str, Handler) {
	("aphrodite_test", |Args| {
		let Resolved = Mode::Resolve::Fn::Fn(Args);
		let Samples = Resolved.Samples();

		let mut Checks = Vec::new();
		let mut Passed = 0usize;

		crate::Session::With::Fn::with_shared(|State| {
			Samples.iter().for_each(|(Content, Hint)| {
				let Info = Compress::Fn::Fn(State, Content, Hint, None);
				let Hash = Info["hash"].as_str().unwrap_or("");
				let Round = aphrodite::Resolve::Expand::Fn(State, Hash);
				let Matched = Round.as_deref() == Some(*Content);

				// A round trip that matched is one more pass: the count is a
				// value, not a branch.
				Passed += usize::from(Matched);

				Checks.push(serde_json::json!({
					"type": Hint, "hash": Hash,
					"roundtrip": Matched,
				}));
			});
		});

		let Status = match Passed == Checks.len() {
			true => "ok",
			false => "fail",
		};
		let Label = match Resolved {
			Mode::Enum::Enum::Quick => "quick",
			Mode::Enum::Enum::Full => "full",
		};

		serde_json::json!({
			"mode": Label,
			"status": Status,
			"passed": Passed,
			"total": Checks.len(),
			"checks": Checks,
			"proxies": proxy_health(),
		})
	})
}
