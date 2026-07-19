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
/// Returns `Some((task_id, command_summary))` if the command/content suggests
/// it should be backgrounded, or `None` to let it pass through normally.
///
/// The caller (agent bridge) is responsible for deciding *which* tools to
/// check — this function only evaluates the command string and content size.
pub fn should_background(
	content: &str,
	command: Option<&str>,
) -> Option<(String, String)> {
	// Short output: pass through normally — the agent doesn't need
	// to poll for a 200-byte `ls`.
	if content.len() < 2048 {
		return None;
	}
	should_background_by_command(command)
}

/// Pre-execution heuristic: check only the command prefix, no content
/// required (we haven't run the tool yet). Used by the `pre_tool_call`
/// hook to decide whether to auto-inject `background=true`.
pub fn should_background_pre(command: Option<&str>) -> Option<(String, String)> {
	should_background_by_command(command)
}

fn should_background_by_command(command: Option<&str>) -> Option<(String, String)> {
	let cmd = command.unwrap_or("").trim();
	if cmd.is_empty() {
		return None;
	}

	let cmd_lower = cmd.to_lowercase();
	for prefix in SLOW_COMMAND_PREFIXES {
		if cmd_lower.starts_with(prefix) {
			let task_id = headroom_core::ccr::compute_key(cmd.as_bytes());
			let summary = cmd.chars().take(60).collect::<String>();
			return Some((task_id, summary));
		}
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

		// Collect task metadata BEFORE mutably borrowing bg_tasks[i],
		// so record_marker can immutably reference the collected values.
		let (output_size, command) = {
			let task = &mut state.bg_tasks[i];
			task.output_hash = Some(hash.clone());
			task.output_preview = Some(preview.clone());
			let sz = task.accumulated_output.len();
			task.accumulated_output.clear();
			let cmd = task.command.clone();
			match exit_code_val {
				0 => task.status = BgStatus::Done { exit_code: 0 },
				_ => task.status = BgStatus::Failed { exit_code: exit_code_val },
			}
			(sz, cmd)
		};

		state.record_marker(MarkerEntry {
			hash,
			ccr_type: "terminal".to_string(),
			size: output_size,
			preview,
			turn: state.turn_counter,
			center: Some(format!("poll:{}", command)),
			meta: None,
		});
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
            &"x".repeat(5000),
            Some("cargo build --release"),
        );
        assert!(result.is_some(), "cargo build should be backgrounded");
    }

    #[test]
    fn test_should_not_background_small_output() {
        let result = should_background(
            "short output",
            Some("cargo build"),
        );
        assert!(result.is_none(), "small output should not be backgrounded");
    }

    #[test]
    fn test_should_background_large_unknown_output() {
        // Unknown command that doesn't match a slow prefix — passes through.
        let result = should_background(
            &"x".repeat(10000),
            Some("some-slow-script.sh"),
        );
        assert!(result.is_none(), "unknown command should NOT be backgrounded (slow-prefix check)");
    }

    #[test]
    fn test_should_not_background_empty_command_small_output() {
        let result = should_background(
            "small",
            None,
        );
        assert!(result.is_none(), "no command + small output should not background");
    }

    #[test]
    fn test_should_background_no_command_very_large_output() {
        // No command = can't determine if slow → passes through.
        let result = should_background(
            &"x".repeat(10000),
            None,
        );
        assert!(result.is_none(), "no-command content should not be backgrounded (needs known prefix)");
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
            10, // started at turn 10, gap=10 < 16 so retain won't drop it
        );
        // Never polled — gap from turn 20 to last_poll 10 = 10 > STALE_TURN_AGE(8).
        state.bg_tasks[0].last_poll_turn = 10;

        expire_stale_tasks(&mut state);
        assert_eq!(
            state.bg_tasks[0].status,
            BgStatus::Stale,
            "unpolled task at turn 20 should be stale (last poll: turn 10, gap=10 > 8)"
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
            let result = should_background(&"x".repeat(5000), Some(&cmd));
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

    // ── Repetition & edge-case tests ──────────────────────────

    #[test]
    fn test_multiple_rapid_polls_accumulate_output() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            "t1".into(),
            "terminal".into(),
            "long-running".into(),
            1,
        );

        // Rapid successive polls — each should accumulate.
        for i in 0..5 {
            let poll_json = serde_json::json!({
                "output": format!("line {}", i),
                "exit_code": serde_json::Value::Null,
            }).to_string();
            let consumed = update_from_poll(&mut state, "process", &poll_json);
            assert!(consumed, "poll {} should be consumed", i);
        }

        assert_eq!(state.bg_tasks[0].poll_count, 5);
        assert!(state.bg_tasks[0].accumulated_output.contains("line 0"));
        assert!(state.bg_tasks[0].accumulated_output.contains("line 4"));
        assert_eq!(state.bg_tasks[0].status, BgStatus::Running);
    }

    #[test]
    fn test_poll_with_exit_code_completes_task() {
        let mut state = AphroditeState::default();
        state.turn_counter = 10;
        insert_bg_task(
            &mut state,
            "t1".into(),
            "terminal".into(),
            "npm install".into(),
            1,
        );

        // First: running poll
        let poll_running = serde_json::json!({
            "output": "installing packages...",
            "exit_code": serde_json::Value::Null,
        }).to_string();
        update_from_poll(&mut state, "process", &poll_running);
        assert_eq!(state.bg_tasks[0].status, BgStatus::Running);

        // Second: completion poll
        let poll_done = serde_json::json!({
            "output": "added 142 packages in 5s",
            "exit_code": 0,
        }).to_string();
        update_from_poll(&mut state, "process", &poll_done);
        assert_eq!(state.bg_tasks[0].status, BgStatus::Done { exit_code: 0 });
        assert!(state.bg_tasks[0].output_hash.is_some());
    }

    #[test]
    fn test_poll_with_text_exit_code_completes_task() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            "t1".into(),
            "terminal".into(),
            "make build".into(),
            1,
        );

        // No JSON wrapper — just raw output with "exit code: 2" in text.
        let consumed = update_from_poll(&mut state, "process", "building...\nfailed!\nexit code: 2\n");
        assert!(consumed);
        assert_eq!(state.bg_tasks[0].status, BgStatus::Failed { exit_code: 2 });
    }

    #[test]
    fn test_accumulated_output_capped_at_256kb() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        insert_bg_task(
            &mut state,
            "t1".into(),
            "terminal".into(),
            "very-verbose".into(),
            1,
        );

        // Push 300KB of output across multiple polls.
        let chunk = "x".repeat(65536); // 64KB
        for _ in 0..5 {
            let poll = serde_json::json!({
                "output": chunk.clone(),
                "exit_code": serde_json::Value::Null,
            }).to_string();
            update_from_poll(&mut state, "process", &poll);
        }

        // Should be capped at ~256KB (256 * 1024).
        assert!(
            state.bg_tasks[0].accumulated_output.len() <= 256 * 1024 + 100,
            "accumulated output must be capped near 256KB, got {}",
            state.bg_tasks[0].accumulated_output.len()
        );
    }

    #[test]
    fn test_bg_task_dedup_prevents_duplicate_running_tasks() {
        let mut state = AphroditeState::default();
        let id = headroom_core::ccr::compute_key(b"cargo build");

        insert_bg_task(&mut state, id.clone(), "terminal".into(), "cargo build".into(), 1);
        insert_bg_task(&mut state, id.clone(), "terminal".into(), "cargo build".into(), 2);
        insert_bg_task(&mut state, id, "terminal".into(), "cargo build".into(), 3);

        assert_eq!(state.bg_tasks.len(), 1, "duplicate running tasks must be deduplicated");
    }

    #[test]
    fn test_bg_task_eviction_prefers_stale_then_done_then_oldest_running() {
        let mut state = AphroditeState::default();
        // Fill to capacity (4) with running tasks.
        for i in 0..4 {
            insert_bg_task(&mut state, format!("t{}", i), "terminal".into(), format!("cmd{}", i), i);
        }
        // Mark t1 as stale.
        state.bg_tasks[1].status = BgStatus::Stale;

        // Insert a 5th — should evict the stale t1.
        insert_bg_task(&mut state, "t5".into(), "terminal".into(), "cmd5".into(), 5);
        assert_eq!(state.bg_tasks.len(), 4);
        let ids: Vec<&str> = state.bg_tasks.iter().map(|t| t.id.as_str()).collect();
        assert!(!ids.contains(&"t1"), "stale task t1 must be evicted first");
        assert!(ids.contains(&"t5"), "new task t5 must be present");

        // Mark t0 as Done, then insert t6 — evicts t0.
        state.bg_tasks.iter_mut().find(|t| t.id == "t0").unwrap().status = BgStatus::Done { exit_code: 0 };
        insert_bg_task(&mut state, "t6".into(), "terminal".into(), "cmd6".into(), 6);
        assert_eq!(state.bg_tasks.len(), 4);
        let ids: Vec<&str> = state.bg_tasks.iter().map(|t| t.id.as_str()).collect();
        assert!(!ids.contains(&"t0"), "done task t0 must be evicted second");

        // All running — insert t7, evicts oldest running (t2).
        insert_bg_task(&mut state, "t7".into(), "terminal".into(), "cmd7".into(), 7);
        let ids: Vec<&str> = state.bg_tasks.iter().map(|t| t.id.as_str()).collect();
        assert!(!ids.contains(&"t2"), "oldest running task t2 must be evicted last");
    }

    #[test]
    fn test_expire_stale_does_not_touch_recently_polled_tasks() {
        let mut state = AphroditeState::default();
        state.turn_counter = 20;
        insert_bg_task(
            &mut state,
            "active".into(),
            "terminal".into(),
            "recent-cmd".into(),
            15,
        );
        state.bg_tasks[0].last_poll_turn = 19; // polled just one turn ago

        expire_stale_tasks(&mut state);
        assert_eq!(
            state.bg_tasks[0].status,
            BgStatus::Running,
            "recently polled task must not be marked stale"
        );
    }

    #[test]
    fn test_check_bg_tasks_respects_2_nudge_cap() {
        let mut state = AphroditeState::default();
        state.turn_counter = 5;
        // Insert 4 running tasks — only 2 should get nudges.
        for i in 0..4 {
            insert_bg_task(
                &mut state,
                format!("t{}", i),
                "terminal".into(),
                format!("cmd{}", i),
                i,
            );
        }

        // Count nudges pushed BEFORE check_bg_tasks.
        let before = state.ephemeral_directives.len();

        check_bg_tasks(&mut state);

        // At most 2 new nudges should be pushed.
        let new_nudges = state.ephemeral_directives.len() - before;
        assert!(
            new_nudges <= 2,
            "must push at most 2 nudges, pushed {}",
            new_nudges
        );
    }

    #[test]
    fn test_create_poll_marker_is_valid_ccr_format() {
        let marker = create_poll_marker("abc123def456abc123def456abc123def456abc123", "cargo build --release", 5);
        // Must start with the preview format
        assert!(marker.contains("[poll:running"), "marker must contain preview");
        // Must end with the CCR marker
        assert!(marker.contains("<<<CCR:"), "marker must contain CCR tag");
        // Must contain the hash
        assert!(marker.contains("abc123def456"), "marker must contain hash");
        // Must contain the type
        assert!(marker.contains("|poll|"), "marker must contain poll type");
    }

    #[test]
    fn test_should_background_empty_command_no_content() {
        let result = should_background("", None);
        assert!(result.is_none(), "empty content should never be backgrounded");
    }

    #[test]
    fn test_should_background_boundary_exactly_2048_bytes() {
        let content = "x".repeat(2048);
        let result = should_background(&content, Some("cargo build"));
        assert!(
            result.is_some(),
            "exactly 2048 bytes of cargo build output should be backgrounded"
        );
    }

    #[test]
    fn test_should_background_boundary_2047_bytes() {
        let content = "x".repeat(2047);
        let result = should_background(&content, Some("cargo build"));
        assert!(
            result.is_none(),
            "2047 bytes should be below the minimum threshold"
        );
    }

    #[test]
    fn test_should_background_no_command_large_output() {
        let result = should_background(&"x".repeat(8192), None);
        assert!(result.is_none(), "no-command output should not be backgrounded (needs known prefix)");
    }

    #[test]
    fn test_render_bg_task_status_with_mixed_states() {
        let mut state = AphroditeState::default();
        insert_bg_task(&mut state, "r1".into(), "terminal".into(), "running1".into(), 1);
        insert_bg_task(&mut state, "r2".into(), "terminal".into(), "running2".into(), 2);
        // Mark one as done.
        state.bg_tasks[1].status = BgStatus::Done { exit_code: 0 };
        insert_bg_task(&mut state, "r3".into(), "terminal".into(), "running3".into(), 3);

        let rendered = render_bg_task_status(&state);
        assert!(rendered.contains("[poll workers]"));
        assert!(rendered.contains("2 active task(s)"), "only running tasks count: {}", rendered);
        // r3 is running, r1 is running = 2 active.
        // r2 is done — not counted.
    }
}
