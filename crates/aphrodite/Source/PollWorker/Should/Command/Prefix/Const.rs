//! The command patterns that strongly suggest long-running work.

/// Known-slow command prefixes.
///
/// Matched case-insensitively against the trimmed command line, so `Cargo Build`
/// and `cargo build` are the same command. The table is a denylist: a command it
/// does not name is never backgrounded.
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
