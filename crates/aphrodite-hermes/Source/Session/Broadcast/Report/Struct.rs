//! What one apply-to-every-session sweep actually did.

/// The reach of one sweep, as counts rather than a verdict.
///
/// The sweep used to answer with a `bool` meaning "some session still has
/// `auto_reload` on", which folded three different situations into one value:
///
///   - every session was reached and every one of them turned `auto_reload`
///     off (the file says stop) - `false`, and stopping is right;
///   - every session's lock was wedged, so the closure never ran and nothing
///     was learned - also `false`, and stopping is WRONG: the watcher would
///     stop because it could not look, which is the same defect class as
///     reporting "unavailable" as "disabled";
///   - the registry was empty (a sweep that ran before the first session
///     registered) - also `false`, and stopping is wrong for the same reason.
///
/// Counting separates them, so the caller decides what a `false` means instead
/// of inheriting a decision the sweep could not make.
pub(crate) struct Struct {
	/// Sessions the sweep reached: the closure ran under their lock, and
	/// whatever it changed is live.
	pub Applied:usize,
	/// Sessions whose bounded lock was unavailable (an abandoned worker holds
	/// it). The closure did NOT run for them, so they learned nothing from this
	/// pass and keep whatever they had.
	pub Skipped:usize,
	/// Of the `Applied` sessions, how many still have `auto_reload` on after
	/// the closure ran. A parse failure does not change this field - the
	/// sessions keep their previous configuration, and the failure is reported
	/// on the state instead.
	pub WantsReload:usize,
}
