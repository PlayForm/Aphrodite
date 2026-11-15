//! `Apply`: an assessment to session state, and the diagnostic to echo back.

use crate::{ABI::Configuration::Reload::Assessment::Enum::Enum as Assessment, State::Struct::AphroditeState};

/// Apply an assessment and answer with the diagnostic the caller should see.
///
/// The four arms are the four situations, and the `None` arms are the three that
/// record nothing. That is what keeps "absent file", "omitted key" and
/// "successful reload" from all collapsing into "clear the error": none of them
/// clears it, and the type says so per variant rather than by a shared default.
///
/// The clone is the genuine fan-out this value has: the same diagnostic is both
/// the session's self-diagnosis and this response's `config_error` field, so the
/// second owner appears here and the copy happens once.
pub(crate) fn Fn(State:&mut AphroditeState, Assessment:&Assessment) -> Option<String> {
	match Assessment {
		Assessment::Ready { Enabled } => {
			State.ContextEngineEnabled = *Enabled;

			None
		},
		Assessment::Invalid { Diagnostic } => {
			State.ConfigurationError = Some(Diagnostic.clone());

			Some(Diagnostic.clone())
		},
		Assessment::Omitted | Assessment::Unreadable => None,
	}
}
