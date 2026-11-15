//! Installing the subscriber exactly once per process.

use crate::Logging::{Install::Fn::Fn as InstallSubscriber, Installed::Const::Installed};

/// Install the global `tracing` subscriber; a second call is a no-op.
///
/// Two callers exist in one process - the binary (`Boot::Run`, after the config
/// is parsed so the configured level and the compact flag are known) and the
/// dylib (`AphroditeInit`) - and a host may have installed a dispatcher of its
/// own before either ran. Both facts are handled here rather than at the call
/// sites: whoever arrives first installs, and everyone after gets `Ok` because
/// the stack they asked for either already exists or was superseded by the
/// host's, which is the host's decision to make.
///
/// `Compact` selects the format and `Configured` is the caller's configured
/// level; both are ignored on a second call, because the first subscriber's
/// shape is the process's shape. That is why the binary installs after the
/// config load and not at the top of `main`: a `--log-compact` run would
/// otherwise be silently upgraded to the structured format by an earlier,
/// env-only install.
///
/// An install that genuinely fails (no global dispatcher, and `try_init`
/// rejected the build) is reported once and then memoised - retrying cannot
/// succeed, and a hot path calling this repeatedly must not pay for the
/// attempt.
pub fn Fn(Compact:bool, Configured:Option<tracing::Level>) -> anyhow::Result<()> {
	let Outcome = Installed.get_or_init(|| {
		match tracing::dispatcher::has_been_set() {
			true => Ok(()),
			false => {
				match InstallSubscriber(Compact, Configured) {
					Ok(()) => Ok(()),
					// Lost a race with another thread between the check above and
					// the install: the stack exists, so this is not a failure.
					Err(Error) => {
						match tracing::dispatcher::has_been_set() {
							true => Ok(()),
							false => Err(format!("{Error:#}")),
						}
					},
				}
			},
		}
	});
	match Outcome {
		Ok(()) => Ok(()),
		Err(Message) => Err(anyhow::anyhow!("{Message}")),
	}
}
