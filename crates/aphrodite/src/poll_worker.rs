//! Poll-worker auto-backgrounding: intercept slow tool calls and replace
//! output with a "poll worker" CCR marker so the agent can check progress
//! asynchronously via `process(action='poll')` instead of blocking on raw
//! output.
//!
//! # Flow
//!
//! 1. A tool call (terminal, process, etc.) produces output.
//! 2. `transform_tool_result` / `transform_terminal_output` calls
//!    `should_background()` — if the heuristic fires, the output is NOT
//!    compressed normally. Instead, a `BgTask` is created and a special
//!    "poll worker" CCR marker replaces the output.
//! 3. `pre_llm_call` → `check_bg_tasks()` pushes ephemeral nudges telling
//!    the agent which tasks are running/done/failed.
//! 4. When the agent calls `process(action='poll')`, the result flows
//!    through `transform_tool_result` again — `update_from_poll()` matches
//!    it to a `BgTask` and updates status.
//! 5. On completion, the final output is compressed to CCR normally so
//!    `aphrodite_retrieve` can resolve it.
//! 6. `post_llm_call` → `expire_stale_tasks()` drops tasks the agent
//!    stopped polling.

use crate::state::{AphroditeState, MarkerEntry};

/// Maximum concurrent background tasks tracked.
const MAX_BG_TASKS: usize = 4;

/// Tasks older than this many turns without a poll are considered stale
/// and auto-expired at `post_llm_call`.
pub const STALE_TURN_AGE: usize = 8;

/// Command patterns that strongly suggest long-running work.
/// Matched case-insensitively against the first whitespace-delimited
/// token of the command line (or the full args if no token boundary).
const SLOW_COMMAND_PREFIXES: &[&str] = &[
    "cargo",
    "npm",
    "yarn",
    "pnpm",
    "pip",
    "pip3",
    "make",
    "cmake",
    "docker",
    "podman",
    "kubectl",
    "terraform",
    "ansible",
    "curl",
    "wget",
    "rsync",
    "git clone",
    "git push",
    "git fetch",
    "brew",
    "apt-get",
    "apt",
    "dnf",
    "pacman",
    "go build",
    "go test",
    "go install",
    "rustup",
    "nix",
    "bazel",
    "gradle",
    "mvn",
];

/// Status of a backgrounded task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgStatus {
    /// Task is still running — agent should keep polling.
    Running,
    /// Task completed successfully.
    Done { exit_code: i32 },
    /// Task failed.
    Failed { exit_code: i32 },
    /// Agent hasn't polled in `STALE_TURN_AGE` turns — abandoned.
    Stale,
}

/// A backgrounded tool call tracked by the poll worker.
#[derive(Debug, Clone)]
pub struct BgTask {
    /// Unique ID: CCR hash of the command string (stable across turns).
    pub id: String,
    /// Tool that produced the output: "terminal" or "process".
    pub tool: String,
    /// The command being run (or tool + args summary for non-terminal).
    pub command: String,
    /// Turn on which the task was auto-backgrounded.
    pub started_turn: usize,
    /// Last turn the agent polled for this task.
    pub last_poll_turn: usize,
    /// Current status.
    pub status: BgStatus,
    /// CCR hash of the compressed output (set when Done/Failed).
    pub output_hash: Option<String>,
    /// CCR preview of the compressed output (set when Done/Failed).
    pub output_preview: Option<String>,
    /// How many times the agent has polled.
    pub poll_count: usize,
    /// The uncompressed output content (accumulated from polls).
    /// Capped to avoid memory blow-up from long builds.
    pub accumulated_output: String,
}

/// Heuristic: should this tool call's output be replaced with a poll-worker
/// marker so the agent can check progress asynchronously?
///
/// Returns `Some((task_id, command_summary))` if the call should be
/// backgrounded, or `None` to let it pass through normally.
pub fn should_background(
    tool_name: &str,
    content: &str,
    command: Option<&str>,
    _args_json: Option<&serde_json::Value>,
) -> Option<(String, String)> {
    // Only terminal and process calls are candidates.
    if tool_name != "terminal" && tool_name != "process" {
        return None;
    }

    // Short output: pass through normally — the agent doesn't need
    // to poll for a 200-byte `ls`.
    if content.len() < 2048 {
        return None;
    }

    let cmd = command.unwrap_or(tool_name).trim();

    // Known slow commands: always background if output is large enough.
    let cmd_lower = cmd.to_lowercase();
    for prefix in SLOW_COMMAND_PREFIXES {
        if cmd_lower.starts_with(prefix) {
            let task_id = headroom_core::ccr::compute_key(cmd.as_bytes());
            let summary = cmd.chars().take(60).collect::<String>();
            return Some((task_id, summary));
        }
    }

    // Large output from unknown commands: background if the output is
    // large enough to suggest it was a long-running task.
    if content.len() >= 8192 {
        let task_id = headroom_core::ccr::compute_key(cmd.as_bytes());
        let summary = cmd.chars().take(60).collect::<String>();
        return Some((task_id, summary));
    }

    None
}

/// Create a "poll worker" CCR marker that replaces the raw tool output.
/// The agent sees this marker instead of the original content and knows
/// to poll for progress rather than retrieve (which would return stale
/// content).
pub fn create_poll_marker(task_id: &str, command_summary: &str, turn: usize) -> String {
    let preview = format!("[poll:running {} | turn {}]", command_summary, turn);
    let hash = task_id;
    crate::marker::ccr_marker(hash, "poll", 0, &preview, None, None, None)
}

/// Check all background tasks and push ephemeral nudges for any that
/// need the agent's attention. Called from `pre_llm_call`.
///
/// Pushes at most 2 nudges (newest-first for active tasks) so the
/// directive/nudge budget isn't consumed by poll workers.
pub fn check_bg_tasks(state: &mut AphroditeState) {
    let turn = state.turn_counter;
    let mut nudged = 0usize;

    // Collect indices to nudge (iterate in insertion order = oldest first,
    // nudging newest active tasks within the 2-nudge cap).
    let task_count = state.bg_tasks.len();
    for i in (0..task_count).rev() {
        if nudged >= 2 {
            break;
        }
        let task = &state.bg_tasks[i];
        match &task.status {
            BgStatus::Running => {
                let last = if task.poll_count > 0 {
                    format!(" (polled {}×, last turn {})", task.poll_count, task.last_poll_turn)
                } else {
                    String::from(" (not polled yet)")
                };
                crate::flow::push_nudge(
                    state,
                    &format!(
                        "poll worker: `{}` still running since turn {}{}. Use process(action='poll').",
                        task.command, task.started_turn, last
                    ),
                    1,
                );
                nudged += 1;
            }
            BgStatus::Done { exit_code } => {
                let hash_preview = task
                    .output_preview
                    .as_deref()
                    .unwrap_or("[output available]");
                crate::flow::push_nudge(
                    state,
                    &format!(
                        "✓ poll worker: `{}` completed (exit {}). Retrieve: aphrodite_retrieve(hash={}) — {}",
                        task.command,
                        exit_code,
                        task.output_hash.as_deref().unwrap_or("?"),
                        hash_preview,
                    ),
                    2, // survive 2 turns so the agent has time to act
                );
                nudged += 1;
            }
            BgStatus::Failed { exit_code } => {
                let hash_preview = task
                    .output_preview
                    .as_deref()
                    .unwrap_or("[error output available]");
                crate::flow::push_nudge(
                    state,
                    &format!(
                        "✗ poll worker: `{}` failed (exit {}). Retrieve: aphrodite_retrieve(hash={}) — {}",
                        task.command,
                        exit_code,
                        task.output_hash.as_deref().unwrap_or("?"),
                        hash_preview,
                    ),
                    2,
                );
                nudged += 1;
            }
            BgStatus::Stale => {
                // Don't nudge about stale tasks — they're abandoned.
            }
        }
    }
}

/// Update a background task's status based on a `process(action='poll')`
/// result flowing through `transform_tool_result`. Called BEFORE the
/// normal compression path so poll output can update the BgTask before
/// anything else happens.
///
/// Returns `true` if the poll result was consumed (matched a bg task),
/// `false` if it should pass through to normal compression.
pub fn update_from_poll(
	state: &mut AphroditeState,
	tool_name: &str,
	content: &str,
) -> bool {
	if tool_name != "process" {
		return false;
	}

	// Try to parse the poll result as JSON (Hermes wraps it).
	let parsed: Option<serde_json::Value> = serde_json::from_str(content).ok();
	let output = parsed
		.as_ref()
		.and_then(|v| v.get("output").and_then(|o| o.as_str()))
		.unwrap_or(content);
	let exit_code = parsed
		.as_ref()
		.and_then(|v| v.get("exit_code").and_then(|e| e.as_i64()))
		.or_else(|| parsed.as_ref().and_then(|v| v.get("returncode").and_then(|e| e.as_i64())));

	// Parse "exit code: N" from the output text (fallback for non-JSON wrappers).
	let text_exit_code: Option<i32> = output
		.lines()
		.rev()
		.find_map(|l| {
			l.split("exit code:")
				.nth(1)
				.and_then(|s| s.trim().split_whitespace().next())
				.and_then(|n| n.parse::<i32>().ok())
		});

	let effective_exit = exit_code.map(|ec| ec as i32).or(text_exit_code);

	let mut matched = false;

	// Phase 1: update task metadata and accumulate output (borrowing bg_tasks).
	for task in state.bg_tasks.iter_mut() {
		if task.status != BgStatus::Running {
			continue;
		}

		task.last_poll_turn = state.turn_counter;
		task.poll_count += 1;

		// Accumulate output (cap at 256KB to avoid unbounded growth).
		if task.accumulated_output.len() < 256 * 1024 {
			if !task.accumulated_output.is_empty() {
				task.accumulated_output.push('\n');
			}
			task.accumulated_output.push_str(output);
		}

		matched = true;
	}

	if !matched {
		return false;
	}

	// Phase 2: check for completions and record CCR markers.
	// Run AFTER the iter_mut() loop to avoid double-borrowing `state`.
	// Collect task indices that need completion recording.
	let mut completions: Vec<(usize, i32, String)> = Vec::new(); // (idx, exit_code, accumulated_output)
	for (i, task) in state.bg_tasks.iter().enumerate() {
		if task.status != BgStatus::Running {
			continue;
		}
		if let Some(ec) = effective_exit {
			completions.push((i, ec, task.accumulated_output.clone()));
		}
	}

	for (i, exit_code_val, acc_output) in completions {
		let hash = headroom_core::ccr::compute_key(acc_output.as_bytes());
		let preview = crate::build_preview("terminal", &acc_output);
		state.inline_store_put(hash.clone(), acc_output);

		let task = &mut state.bg_tasks[i];
		task.output_hash = Some(hash.clone());
		task.output_preview = Some(preview.clone());
		task.accumulated_output.clear();

		state.record_marker(MarkerEntry {
			hash,
			ccr_type: "terminal".to_string(),
			size: task.accumulated_output.len(),
			preview,
			turn: state.turn_counter,
			center: Some(format!("poll:{}", task.command)),
			meta: None,
		});

		match exit_code_val {
			0 => task.status = BgStatus::Done { exit_code: 0 },
			_ => task.status = BgStatus::Failed { exit_code: exit_code_val },
		}
	}

	true
}

/// Expire background tasks that the agent has stopped polling.
/// Called from `post_llm_call` after turn advancement.
pub fn expire_stale_tasks(state: &mut AphroditeState) {
    let turn = state.turn_counter;
    for task in state.bg_tasks.iter_mut() {
        if task.status == BgStatus::Running
            && turn.saturating_sub(task.last_poll_turn) > STALE_TURN_AGE
        {
            task.status = BgStatus::Stale;
        }
    }

    // Drop completed/failed/stale tasks older than 16 turns.
    state.bg_tasks.retain(|task| {
        match task.status {
            BgStatus::Running => true, // keep running tasks
            BgStatus::Done { .. } | BgStatus::Failed { .. } | BgStatus::Stale => {
                turn.saturating_sub(task.started_turn) <= 16
            }
        }
    });
}

/// Insert a new background task into the state, evicting the oldest
/// completed/stale task if at capacity.
pub fn insert_bg_task(
    state: &mut AphroditeState,
    task_id: String,
    tool: String,
    command: String,
    turn: usize,
) {
    // Deduplicate: don't track the same command twice while it's running.
    if state
        .bg_tasks
        .iter()
        .any(|t| t.id == task_id && t.status == BgStatus::Running)
    {
        return;
    }

    // Evict oldest non-running task if at capacity.
    while state.bg_tasks.len() >= MAX_BG_TASKS {
        // Prefer evicting stale, then done/failed.
        let stale_pos = state
            .bg_tasks
            .iter()
            .position(|t| t.status == BgStatus::Stale);
        let done_pos = state
            .bg_tasks
            .iter()
            .position(|t| matches!(t.status, BgStatus::Done { .. } | BgStatus::Failed { .. }));
        let pos = stale_pos.or(done_pos);
        if let Some(p) = pos {
            state.bg_tasks.remove(p);
        } else {
            // All running — drop the oldest running task.
            state.bg_tasks.remove(0);
        }
    }

    state.bg_tasks.push_back(BgTask {
        id: task_id,
        tool,
        command,
        started_turn: turn,
        last_poll_turn: turn,
        status: BgStatus::Running,
        output_hash: None,
        output_preview: None,
        poll_count: 0,
        accumulated_output: String::new(),
    });
}

/// Render bg task status lines for injection into `build_turn_context`.
/// Returns empty string if no active tasks.
pub fn render_bg_task_status(state: &AphroditeState) -> String {
    let active: Vec<&BgTask> = state
        .bg_tasks
        .iter()
        .filter(|t| t.status == BgStatus::Running)
        .collect();

    if active.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("[poll workers] {} active task(s):", active.len())];
    for task in active.iter().take(3) {
        let poll_info = if task.poll_count > 0 {
            format!("polled {}×", task.poll_count)
        } else {
            "not polled yet".to_string()
        };
        lines.push(format!(
            "  · `{}` — running since turn {} ({})",
            task.command, task.started_turn, poll_info
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AphroditeState;

    #[test]
    fn test_should_background_slow_command() {
        let result = should_background(
            "terminal",
            &"x".repeat(5000),
            Some("cargo build --release"),
            None,
        );
        assert!(result.is_some(), "cargo build should be backgrounded");
    }

    #[test]
    fn test_should_not_background_small_output() {
        let result = should_background(
            "terminal",
            "short output",
            Some("cargo build"),
            None,
        );
        assert!(result.is_none(), "small output should not be backgrounded");
    }

    #[test]
    fn test_should_not_background_read_file() {
        let result = should_background(
            "read_file",
            &"x".repeat(5000),
            None,
            None,
        );
        assert!(result.is_none(), "read_file should never be backgrounded");
    }

    #[test]
    fn test_should_background_large_unknown_output() {
        let result = should_background(
            "terminal",
            &"x".repeat(10000),
            Some("some-slow-script.sh"),
            None,
        );
        assert!(result.is_some(), "large unknown output should be backgrounded");
    }

    #[test]
    fn test_insert_bg_task_dedup() {
        let mut state = AphroditeState::default();
        insert_bg_task(
            &mut state,
            "task1".into(),
            "terminal".into(),
            "cargo build".into(),
            1,
        );
        assert_eq!(state.bg_tasks.len(), 1);

        // Same ID, still running — should dedup.
        insert_bg_task(
            &mut state,
            "task1".into(),
            "terminal".into(),
            "cargo build".into(),
            2,
        );
        assert_eq!(state.bg_tasks.len(), 1);
    }

    #[test]
    fn test_insert_bg_task_eviction_at_capacity() {
        let mut state = AphroditeState::default();
        for i in 0..MAX_BG_TASKS + 1 {
            insert_bg_task(
                &mut state,
                format!("task{i}"),
                "terminal".into(),
                format!("cmd{i}"),
                i,
            );
        }
        assert_eq!(
            state.bg_tasks.len(),
            MAX_BG_TASKS,
            "must not exceed MAX_BG_TASKS"
        );
    }

    #[test]
    fn test_expire_stale_tasks() {
        let mut state = AphroditeState::default();
        state.turn_counter = 20;
        insert_bg_task(
            &mut state,
            "stale".into(),
            "terminal".into(),
            "old cmd".into(),
            1,
        );
        // Simulate running since turn 1, never polled.
        state.bg_tasks[0].last_poll_turn = 1;

        expire_stale_tasks(&mut state);
        assert_eq!(
            state.bg_tasks[0].status,
            BgStatus::Stale,
            "unpolled task at turn 20 should be stale (last poll: turn 1)"
        );
    }

    #[test]
    fn test_render_bg_task_status_empty() {
        let state = AphroditeState::default();
        assert_eq!(render_bg_task_status(&state), "");
    }

    #[test]
    fn test_render_bg_task_status_with_tasks() {
        let mut state = AphroditeState::default();
        insert_bg_task(
            &mut state,
            "task1".into(),
            "terminal".into(),
            "cargo build".into(),
            5,
        );
        insert_bg_task(
            &mut state,
            "task2".into(),
            "terminal".into(),
            "npm install".into(),
            6,
        );
        let rendered = render_bg_task_status(&state);
        assert!(rendered.contains("[poll workers]"));
        assert!(rendered.contains("cargo build"));
        assert!(rendered.contains("npm install"));
        assert!(rendered.contains("not polled yet"));
    }

    #[test]
    fn test_update_from_poll_exit_code_zero() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            headroom_core::ccr::compute_key(b"cargo build"),
            "terminal".into(),
            "cargo build".into(),
            1,
        );

        let poll_json = serde_json::json!({
            "output": "Compiling...\nFinished\nexit code: 0",
            "exit_code": 0
        })
        .to_string();

        let consumed = update_from_poll(&mut state, "process", &poll_json);
        assert!(consumed, "poll result should be consumed");
        assert_eq!(state.bg_tasks[0].status, BgStatus::Done { exit_code: 0 });
        assert!(state.bg_tasks[0].output_hash.is_some());
        assert!(state.bg_tasks[0].output_preview.is_some());
    }

    #[test]
    fn test_update_from_poll_exit_code_nonzero() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            headroom_core::ccr::compute_key(b"cargo build"),
            "terminal".into(),
            "cargo build".into(),
            1,
        );

        let poll_json = serde_json::json!({
            "output": "error: compilation failed\nexit code: 1",
            "exit_code": 1
        })
        .to_string();

        let consumed = update_from_poll(&mut state, "process", &poll_json);
        assert!(consumed);
        assert_eq!(state.bg_tasks[0].status, BgStatus::Failed { exit_code: 1 });
    }

    #[test]
    fn test_update_from_poll_no_exit_code_still_running() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            headroom_core::ccr::compute_key(b"cargo build"),
            "terminal".into(),
            "cargo build".into(),
            1,
        );

        let poll_json = serde_json::json!({
            "output": "still compiling...",
            "exit_code": null
        })
        .to_string();

        let consumed = update_from_poll(&mut state, "process", &poll_json);
        assert!(consumed);
        assert_eq!(state.bg_tasks[0].status, BgStatus::Running);
        assert_eq!(state.bg_tasks[0].poll_count, 1);
        assert!(state.bg_tasks[0].accumulated_output.contains("still compiling"));
    }

    #[test]
    fn test_create_poll_marker_format() {
        let marker = create_poll_marker("abc123def456", "cargo build", 5);
        assert!(marker.contains("<<<CCR:"));
        assert!(marker.contains("poll"));
        assert!(marker.contains("cargo build"));
        assert!(marker.contains("turn 5"));
    }

    #[test]
    fn test_should_background_various_slow_prefixes() {
        for prefix in SLOW_COMMAND_PREFIXES.iter().take(5) {
            let cmd = format!("{} --some-flag", prefix);
            let result = should_background("terminal", &"x".repeat(5000), Some(&cmd), None);
            assert!(
                result.is_some(),
                "prefix '{}' should trigger backgrounding for command: {}",
                prefix,
                cmd
            );
        }
    }

    #[test]
    fn test_check_bg_tasks_pushes_nudge_for_running() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            "t1".into(),
            "terminal".into(),
            "cargo build".into(),
            1,
        );

        check_bg_tasks(&mut state);
        assert!(
            state.ephemeral_directives.len() >= 1,
            "should push at least one nudge"
        );
        let nudge = state.ephemeral_directives.last().unwrap();
        assert!(
            nudge.inline.as_deref().unwrap().contains("cargo build"),
            "nudge should mention the command"
        );
    }

    #[test]
    fn test_check_bg_tasks_pushes_completion_nudge() {
        let mut state = AphroditeState::default();
        state.turn_counter = 6;
        insert_bg_task(
            &mut state,
            "t1".into(),
            "terminal".into(),
            "cargo build".into(),
            1,
        );
        state.bg_tasks[0].status = BgStatus::Done { exit_code: 0 };
        state.bg_tasks[0].output_hash = Some("hash123".into());
        state.bg_tasks[0].output_preview = Some("[build:0E 2W 450L]".into());

        check_bg_tasks(&mut state);
        let nudge = state.ephemeral_directives.last().unwrap();
        assert!(
            nudge
                .inline
                .as_deref()
                .unwrap()
                .contains("completed"),
            "completion nudge should say 'completed'"
        );
        assert!(
            nudge.inline.as_deref().unwrap().contains("hash123"),
            "completion nudge should include hash"
        );
    }
}
