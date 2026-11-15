//! The command-prefix table and the matcher that consults it.
//!
//! This is the only place that knows which commands are presumed slow. It
//! is a denylist of known-slow prefixes, not a timer: an unknown command is
//! never backgrounded however large its output, because backgrounding
//! something that already finished would hand the agent a marker to poll for
//! work that is done. The match is case-insensitive against the trimmed
//! command line.

/// Command patterns that strongly suggest long-running work.
/// Matched case-insensitively against the first whitespace-delimited
/// token of the command line (or the full args if no token boundary).
pub(crate) const SlowCommandPrefixes:&[&str] = &[
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

/// Match `command` against `SlowCommandPrefixes` and mint the task
/// identity when it hits.
///
/// Returns `None` for an empty or absent command - a command the poll worker
/// cannot see is a command it must not background, since the resulting task
/// could never be matched back to a poll.
pub(crate) fn ShouldBackgroundByCommand(command:Option<&str>) -> Option<(String, String)> {
	let Command = command.unwrap_or("").trim();
	if Command.is_empty() {
		return None;
	}

	let CommandLower = Command.to_lowercase();
	for Prefix in SlowCommandPrefixes {
		if CommandLower.starts_with(Prefix) {
			let TaskID = headroom_core::ccr::compute_key(Command.as_bytes());
			let Summary = Command.chars().take(60).collect::<String>();
			return Some((TaskID, Summary));
		}
	}

	None
}
