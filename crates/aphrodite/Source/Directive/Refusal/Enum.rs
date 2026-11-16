//! The two refusals the directive vocabulary can answer.

/// Why a directive action was refused.
///
/// The vocabulary refuses in exactly two places, and they are different questions: the ACTION
/// is not one the engine knows, or the NAME is not a directive it has loaded. Both used to be
/// spelled inline where they happened - the unknown action in the dispatcher's catch-all arm,
/// the unknown directive in a verb leaf - so "what an unknown directive does" had to be read
/// out of three files.
///
/// The tolerance policy for directives is therefore one table: an unknown ACTION is refused and
/// the message lists the vocabulary; an unknown NAME is refused by the verbs that asked for
/// exactly that directive (`swap`, `load`), and silently ignored by `add`, which pre-seeds a set
/// and treats a stale name as a caller's list rather than a user's typo. `add`'s silence is a
/// deliberate row of this table, not an oversight - see [`super::super::Verb::Add::Fn::Fn`].
pub(crate) enum Enum {
	/// The action vocabulary has no such verb.
	Action,
	/// A verb that needs a loaded directive was given a name that is not loaded.
	Name,
}
