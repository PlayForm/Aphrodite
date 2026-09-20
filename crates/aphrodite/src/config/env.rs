/// Resolve a boolean env var with one consistent truthiness rule across the
/// crate (report 07 F12): `"1"`/`"true"` (case-insensitive) is true,
/// anything else present (including `"0"`/`"false"`/empty string) or absent
/// is false. Previously this repo had three different ad-hoc conventions in
/// three places: `"true"/"1"` (the dead `config_loader`), `"1"` only
/// (Python's `APHRODITE_CONTEXT_ENGINE` check), and presence-only
/// (`APHRODITE_LOG_COMPACT=0` used to still enable compact logging, since
/// `.is_ok()` doesn't look at the value at all).
pub fn env_bool(var:&str) -> bool {
	match std::env::var(var) {
		Ok(v) => matches!(v.to_lowercase().as_str(), "1" | "true"),
		Err(_) => false,
	}
}

/// Parse a present env var, warning (not silently defaulting) if it fails to
/// parse as `T` - the bug class `Maintain/examples/01_env_var_typo.py`
/// documents and `MultiConfig::apply_port_override`'s comment explains
/// (report 07 F10/F15): a missing var is the unremarkable common case, but a
/// *present-and-malformed* one left an operator with no way to tell "my
/// override didn't apply" from "I didn't set an override". Single shared
/// implementation so every numeric env-var read in the crate uses the same
/// rule (report 07 §7 generalization note).
pub fn env_parse_warn<T:std::str::FromStr>(var:&str) -> Option<T> {
	match std::env::var(var) {
		Ok(v) => {
			match v.parse::<T>() {
				Ok(parsed) => Some(parsed),
				Err(_) => {
					tracing::warn!("{}={:?} could not be parsed; ignoring override", var, v);
					None
				},
			}
		},
		Err(_) => None,
	}
}
