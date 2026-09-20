//! Runtime directive actions (`list`/`swap`/`add`/`remove`/`reset`/`load`)
//! against live state.

/// Handle a directive action (`list`/`swap`/`add`/`remove`/`reset`/`load`)
/// against live state. Both `aphrodite_directive` (extern fn) and
/// `aphrodite_dispatch`'s `"directive"` arm delegate here (01-F8) - previously
/// ~40 lines of this logic were duplicated between the two, with divergent
/// error shapes (the extern fn returned an error via a separate top-level
/// `to_json_error` call, the dispatch arm embedded `{"error": ...}` inside an
/// otherwise-success value). This always returns the latter shape - callers
/// pass the result straight through their own success serializer.
pub fn handle_action(state:&mut crate::state::AphroditeState, action:&str, name:&str) -> serde_json::Value {
	match action {
		"list" => {
			// P3/T10: surface ephemeral (nudge/TTL) entries with their expiry so
			// the mechanism is observable.
			let ephemeral:Vec<serde_json::Value> = state
				.ephemeral_directives
				.iter()
				.map(|e| {
					serde_json::json!({
						"name": e.name,
						"inline": e.inline,
						"expires_after_turn": e.expires_after_turn,
					})
				})
				.collect();
			let mut available:Vec<&String> = state.directives.keys().collect();
			// Sort for a stable, deterministic ordering - `HashMap` iteration
			// order is nondeterministic, which made the `list` result flake
			// between `["focus","lazy"]` and `["lazy","focus"]` across runs.
			available.sort();
			serde_json::json!({
				"available": available,
				"active": &state.active_directives,
				"ephemeral": ephemeral,
			})
		},
		"swap" => {
			if state.directives.contains_key(name) {
				state.active_directives = vec![name.to_string()];
				state.manual_directive_turn = Some(state.turn_counter);
				serde_json::json!({"swapped": name, "active": &state.active_directives})
			} else {
				serde_json::json!({"error": format!("unknown directive: {}", name)})
			}
		},
		"add" => {
			if state.directives.contains_key(name) && !state.active_directives.contains(&name.to_string()) {
				state.active_directives.push(name.to_string());
			}
			state.manual_directive_turn = Some(state.turn_counter);
			serde_json::json!({"active": &state.active_directives})
		},
		// ── "load" (lazy directive activation, 06-T?) - activate a directive
		// from the *available* set on demand, returning a distinct shape so
		// the caller can tell a lazy load apart from a pre-seeded `add`.
		// Unlike `add` it is NOT silent on an unknown name: it errors, since a
		// lazy load is an explicit, intentional activation and a typo should
		// surface rather than be swallowed. Idempotent if already active.
		"load" => {
			if !state.directives.contains_key(name) {
				return serde_json::json!({"error": format!("unknown directive: {}", name)});
			}
			if !state.active_directives.contains(&name.to_string()) {
				state.active_directives.push(name.to_string());
			}
			state.manual_directive_turn = Some(state.turn_counter);
			serde_json::json!({"loaded": name, "active": &state.active_directives})
		},
		"remove" => {
			state.active_directives.retain(|d| d != name);
			state.manual_directive_turn = Some(state.turn_counter);
			serde_json::json!({"active": &state.active_directives})
		},
		"reset" => {
			// `reset` clears named actives AND ephemeral nudges AND the manual
			// latch (P3/P6: explicit return to auto mode - see T10/T20).
			state.active_directives.clear();
			state.ephemeral_directives.clear();
			state.manual_directive_turn = None;
			serde_json::json!({"active": &state.active_directives})
		},
		_ => serde_json::json!({"error": format!("unknown action: {} (use list|swap|add|load|remove|reset)", action)}),
	}
}
