//! Numeric environment variables.

/// Parse a present env var, warning (not silently defaulting) if it fails to
/// parse as `T` - the bug class `Maintain/examples/01_env_var_typo.py`
/// documents and `MultiConfig::apply_port_override`'s comment explains
/// (report 07 F10/F15): a missing var is the unremarkable common case, but a
/// *present-and-malformed* one left an operator with no way to tell "my
/// override didn't apply" from "I didn't set an override". Single shared
/// implementation so every numeric env-var read in the crate uses the same
/// rule (report 07 §7 generalization note).
///
/// `None` therefore means two different things - absent, or present and
/// unusable - and the difference is only visible in the log. Callers chain
/// `.or(toml_value).unwrap_or(default)` on top of it, so a typo'd override
/// degrades to the file's value rather than to a hard failure.
pub fn EnvironmentParseWarn<T:std::str::FromStr>(Var:&str) -> Option<T> {
	match std::env::var(Var) {
		Ok(v) => {
			match v.parse::<T>() {
				Ok(Parsed) => Some(Parsed),
				Err(_) => {
					tracing::warn!("{}={:?} could not be parsed; ignoring override", Var, v);
					None
				},
			}
		},
		Err(_) => None,
	}
}
