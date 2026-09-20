/// An ephemeral directive activation entry (P3/T9). Named entries key into
/// `state.directives`; inline entries carry literal nudge text synthesized by a
/// feature (error-loop breaker, redundant-read deflector, auto-swap announce).
/// `expires_after_turn = None` is permanent; `Some(n)` renders while
/// `turn_counter <= n` and is purged in `post_llm_call` once past.
#[derive(Debug, Clone)]
pub struct ActiveDirective {
	/// Key into `state.directives`, or empty for an inline entry.
	pub name:String,
	/// Literal nudge text for synthesized entries (rendered as `[nudge: …]`).
	pub inline:Option<String>,
	/// Last turn on which this entry renders; `None` = permanent.
	pub expires_after_turn:Option<usize>,
}
