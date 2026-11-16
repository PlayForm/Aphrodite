//! Boolean environment variables.

/// Resolve a boolean env var with one consistent truthiness rule across the
/// crate (report 07 F12): `"1"`/`"true"` (case-insensitive) is true,
/// anything else present (including `"0"`/`"false"`/empty string) or absent
/// is false. Previously this repo had three different ad-hoc conventions in
/// three places: `"true"/"1"` (the dead `config_loader`), `"1"` only
/// (Python's `APHRODITE_CONTEXT_ENGINE` check), and presence-only
/// (`APHRODITE_LOG_COMPACT=0` used to still enable compact logging, since
/// `.is_ok()` doesn't look at the value at all).
///
/// Present-and-false and absent are indistinguishable here, deliberately:
/// callers that need to tell "explicitly off" from "unset" must read the
/// variable themselves.
pub fn Fn(Var:&str) -> bool {
	match std::env::var(Var) {
		Ok(Value) => matches!(Value.to_lowercase().as_str(), "1" | "true"),
		Err(_) => false,
	}
}
