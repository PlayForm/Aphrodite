//! The tool bypass list.

/// Essential tools that must NOT be compressed - agent needs raw output.
///
/// The list is a behavioural contract, not a tuning knob: `read_file` and
/// `skill_view` return the content the agent is about to act on, so replacing
/// them with a marker would force a retrieval before every edit. `read_file`
/// is also a tracked file tool - skipping compression and recording the file
/// reference are independent decisions, and it is in both lists on purpose.
pub(crate) const Essential:&[&str] = &[
	"skill_view",
	"skills_list",
	"skill_manage",
	"memory",
	"session_search",
	"read_file",
	"read_terminal",
];
