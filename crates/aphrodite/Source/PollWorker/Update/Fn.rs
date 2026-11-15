//! Poll-result ingestion: turn a `process(action='poll')` result into task
//! progress, and finish the task when an exit code finally appears.
//!
//! Two phases, and the split is load-bearing: the first mutates every running
//! task while borrowing `BgTasks`, the second records CCR markers and needs
//! `&mut AphroditeState`. Doing it in one pass double-borrows the state. The
//! exit code is read from the JSON wrapper when present and from an
//! `exit code: N` line otherwise, because not every poll result is wrapped.

use crate::{
	PollWorker::Task::Status::Enum::BgStatus,
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Update a background task's status based on a `process(action='poll')`
/// result flowing through `TransformToolResult`. Called BEFORE the
/// normal compression path so poll output can update the BgTask before
/// anything else happens.
///
/// Returns `true` if the poll result was consumed (matched a bg task),
/// `false` if it should pass through to normal compression.
pub fn UpdateFromPoll(State:&mut AphroditeState, ToolName:&str, Content:&str) -> bool {
	if ToolName != "process" {
		return false;
	}

	// Try to parse the poll result as JSON (Hermes wraps it).
	let Parsed:Option<serde_json::Value> = serde_json::from_str(Content).ok();
	let Output = Parsed
		.as_ref()
		.and_then(|v| v.get("output").and_then(|o| o.as_str()))
		.unwrap_or(Content);
	let ExitCode = Parsed
		.as_ref()
		.and_then(|v| v.get("exit_code").and_then(|e| e.as_i64()))
		.or_else(|| Parsed.as_ref().and_then(|v| v.get("returncode").and_then(|e| e.as_i64())));

	// Parse "exit code: N" from the output text (fallback for non-JSON wrappers).
	let TextExitCode:Option<i32> = Output.lines().rev().find_map(|l| {
		l.split("exit code:")
			.nth(1)
			.and_then(|s| s.split_whitespace().next())
			.and_then(|n| n.parse::<i32>().ok())
	});

	let EffectiveExit = ExitCode.map(|Ec| Ec as i32).or(TextExitCode);

	let mut Matched = false;

	// Phase 1: update task metadata and accumulate output (borrowing bg_tasks).
	for task in State.BgTasks.iter_mut() {
		if task.Status != BgStatus::Running {
			continue;
		}

		task.LastPollTurn = State.TurnCounter;
		task.PollCount += 1;

		// Accumulate output (cap at 256KB to avoid unbounded growth).
		if task.AccumulatedOutput.len() < 256 * 1024 {
			if !task.AccumulatedOutput.is_empty() {
				task.AccumulatedOutput.push('\n');
			}
			task.AccumulatedOutput.push_str(Output);
		}

		// Live-container progress (pre-Rust `_hooks/live.py` container
		// semantics): bytes-so-far is the accumulated output; while still
		// running, the ETA is the elapsed-so-far heuristic (no output
		// total is known for a backgrounded command).
		task.LiveBytesArrived = task.AccumulatedOutput.len();
		let Elapsed = crate::Prefetch::Clock::Fn::UnixNow().saturating_sub(task.LiveArrivedAtUnix) as f32;
		task.LiveEtaS = Elapsed;

		Matched = true;
	}

	if !Matched {
		return false;
	}

	// Phase 2: check for completions and record CCR markers.
	// Run AFTER the iter_mut() loop to avoid double-borrowing `State`.
	// Collect task indices that need completion recording.
	let mut Completions:Vec<(usize, i32, String)> = Vec::new(); // (idx, exit_code, accumulated_output)
	for (i, task) in State.BgTasks.iter().enumerate() {
		if task.Status != BgStatus::Running {
			continue;
		}
		if let Some(Ec) = EffectiveExit {
			Completions.push((i, Ec, task.AccumulatedOutput.clone()));
		}
	}

	for (i, ExitCodeValue, AccOutput) in Completions {
		let hash = headroom_core::ccr::compute_key(AccOutput.as_bytes());
		let Preview = crate::Preview::Builder::Entry::Fn("terminal", &AccOutput);
		State.InlineStorePut(hash.clone(), AccOutput);

		// Collect task metadata BEFORE mutably borrowing bg_tasks[i],
		// so record_marker can immutably reference the collected values.
		let (OutputSize, command) = {
			let task = &mut State.BgTasks[i];
			task.OutputHash = Some(hash.clone());
			task.OutputPreview = Some(Preview.clone());
			let Sz = task.AccumulatedOutput.len();
			// Live-container progress: the full content has arrived, so
			// bytes-so-far is final and the ETA is zero.
			task.LiveBytesArrived = Sz;
			task.LiveEtaS = 0.0;
			task.AccumulatedOutput.clear();
			let Command = task.command.clone();
			match ExitCodeValue {
				0 => task.Status = BgStatus::Done { ExitCode:0 },
				_ => task.Status = BgStatus::Failed { ExitCode:ExitCodeValue },
			}
			(Sz, Command)
		};

		State.RecordMarker(MarkerEntry {
			hash,
			CCRType:"terminal".to_string(),
			Size:OutputSize,
			Preview,
			Turn:State.TurnCounter,
			Center:Some(format!("poll:{}", command)),
			meta:None,
		});
	}

	true
}
