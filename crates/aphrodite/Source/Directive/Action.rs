//! Runtime directive actions (`list`/`swap`/`add`/`remove`/`reset`/`Load`)
//! against live state.
//!
//! This is the single implementation of the action vocabulary. Both
//! `AphroditeDirective` (the extern fn) and `AphroditeDispatch`'s
//! `"directive"` arm delegate here (01-F8) - previously ~40 lines of this
//! logic were duplicated between the two, with divergent error shapes. The
//! returned value always embeds an `{"error": ...}` key rather than a separate
//! top-level error call, so a caller can pass it straight through its own
//! success serializer.
//!
//! Every successful mutation latches `ManualDirectiveTurn` (P6): a manual
//! choice must switch phase-aware auto-swaps off, and a *failed* action must
//! not, which is why the latch sits inside each success arm rather than at the
//! top of the function.

/// Handle a directive action (`list`/`swap`/`add`/`remove`/`reset`/`Load`)
/// against live state. Both `AphroditeDirective` (extern fn) and
/// `AphroditeDispatch`'s `"directive"` arm delegate here (01-F8) - previously
/// ~40 lines of this logic were duplicated between the two, with divergent
/// error shapes (the extern fn returned an error via a separate top-level
/// `ToJSONError` call, the dispatch arm embedded `{"error": ...}` inside an
/// otherwise-success value). This always returns the latter shape - callers
/// pass the result straight through their own success serializer.
pub fn Fn(State:&mut crate::State::Struct::AphroditeState, Action:&str, name:&str) -> serde_json::Value {
	match Action {
		"list" => {
			// P3/T10: surface ephemeral (nudge/TTL) entries with their expiry so
			// the mechanism is observable.
			let Ephemeral:Vec<serde_json::Value> = State
				.EphemeralDirectives
				.iter()
				.map(|e| {
					serde_json::json!({
						"name": e.name,
						"inline": e.inline,
						"expires_after_turn": e.ExpiresAfterTurn,
					})
				})
				.collect();
			let mut Available:Vec<&String> = State.Directives.keys().collect();
			// Sort for a stable, deterministic ordering - `HashMap` iteration
			// order is nondeterministic, which made the `list` result flake
			// between `["focus","lazy"]` and `["lazy","focus"]` across runs.
			Available.sort();
			serde_json::json!({
				"available": Available,
				"active": &State.ActiveDirectives,
				"ephemeral": Ephemeral,
			})
		},
		"swap" => {
			if State.Directives.contains_key(name) {
				State.ActiveDirectives = vec![name.to_string()];
				State.ManualDirectiveTurn = Some(State.TurnCounter);
				serde_json::json!({"swapped": name, "active": &State.ActiveDirectives})
			} else {
				serde_json::json!({"error": format!("unknown directive: {}", name)})
			}
		},
		"add" => {
			if State.Directives.contains_key(name) && !State.ActiveDirectives.iter().any(|d| d == name) {
				State.ActiveDirectives.push(name.to_string());
			}
			State.ManualDirectiveTurn = Some(State.TurnCounter);
			serde_json::json!({"active": &State.ActiveDirectives})
		},
		// ── "load" (lazy directive activation, 06-T?) - activate a directive
		// from the *available* set on demand, returning a distinct shape so
		// the caller can tell a lazy load apart from a pre-seeded `add`.
		// Unlike `add` it is NOT silent on an unknown name: it errors, since a
		// lazy load is an explicit, intentional activation and a typo should
		// surface rather than be swallowed. Idempotent if already active.
		"load" => {
			if !State.Directives.contains_key(name) {
				return serde_json::json!({"error": format!("unknown directive: {}", name)});
			}
			if !State.ActiveDirectives.iter().any(|d| d == name) {
				State.ActiveDirectives.push(name.to_string());
			}
			State.ManualDirectiveTurn = Some(State.TurnCounter);
			serde_json::json!({"loaded": name, "active": &State.ActiveDirectives})
		},
		"remove" => {
			State.ActiveDirectives.retain(|d| d != name);
			State.ManualDirectiveTurn = Some(State.TurnCounter);
			serde_json::json!({"active": &State.ActiveDirectives})
		},
		"reset" => {
			// `reset` clears named actives AND ephemeral nudges AND the manual
			// latch (P3/P6: explicit return to auto mode - see T10/T20).
			State.ActiveDirectives.clear();
			State.EphemeralDirectives.clear();
			State.ManualDirectiveTurn = None;
			serde_json::json!({"active": &State.ActiveDirectives})
		},
		_ => serde_json::json!({"error": format!("unknown action: {} (use list|swap|add|load|remove|reset)", Action)}),
	}
}
