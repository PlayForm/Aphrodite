# Security Policy for Aphrodite&#x2001;💋

The PlayForm team and the Aphrodite community are committed to the security of
the CCR compression proxy and its Hermes Agent integration. Aphrodite runs on
the user's machine, sits between the agent and the model provider, and keeps
the prompt and tool output it compresses in a local store, so a weakness here
can expose conversation content, tool output, or the credentials that pay for
the model.

This policy covers this repository and both halves it ships: the proxy binary
in [`crates/aphrodite`](https://github.com/PlayForm/Aphrodite/tree/Development/crates/aphrodite)
and the Hermes plugin, which is the Rust dylib in
[`crates/aphrodite-hermes`](https://github.com/PlayForm/Aphrodite/tree/Development/crates/aphrodite-hermes)
plus the Python shim in
[`plugins/aphrodite`](https://github.com/PlayForm/Aphrodite/tree/Development/plugins/aphrodite).
The plugin also ships from
[`PlayForm/Aphrodite-Hermes`](https://github.com/PlayForm/Aphrodite-Hermes).

Your reports help us keep that environment safe for every user. Please follow
this policy so that a finding reaches the people who can fix it before it
becomes public.

> [!IMPORTANT]
>
> Report privately by email to
> [Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud), never through a
> public issue, pull request, or discussion.

## Guiding Principles&#x2001;🧭

- **Proactive Security:** We build security into the proxy, the dylib, and the
  plugin from the ground up rather than bolting it on after a report.
- **Responsible Disclosure:** We ask for private reporting so a finding is not
  exposed before a fix exists.
- **Timely Response:** We acknowledge, triage, and fix reported issues on the
  schedule published below.
- **Transparency:** Once a fix ships we publish an advisory and credit the
  reporter, unless the reporter asks for anonymity.
- **Collaboration:** We work with reporters, upstream dependencies, and Hermes
  maintainers instead of treating a report as a contest.

## Scope of this Policy&#x2001;🎯

This policy applies to vulnerabilities discovered within:

1. **The codebase of this repository.** This includes the proxy and engine
   (`crates/aphrodite`), the FFI dylib and its tools (`crates/aphrodite-hermes`),
   the plugin shim (`plugins/aphrodite`), the maintenance scripts
   (`Maintain/`), and the build and release workflows (`.github/workflows`).

2. **Direct dependencies bundled or vendored here.** This includes the bundled
   SQLite build and the vendored `headroom-core` crate under
   [`vendor/headroom`](https://github.com/PlayForm/Aphrodite/tree/Development/vendor/headroom).

3. **The interaction points this project exposes.** The loopback HTTP proxy on
   ports 9797 and 9798, its management routes, the FFI boundary into the Hermes
   process, the plugin shim, and the release artifacts with their
   `SHA256SUMS` files.

Vulnerabilities in Hermes Agent itself, in the model provider, or in the
operating system are outside this policy; report those upstream. If Aphrodite
is the vector, tell us as well and we will coordinate.

**Examples of vulnerabilities relevant to Aphrodite include (but are not
limited to):**

- **Remote Code Execution (RCE):** A crafted config, template, directive, or
  Rhai script that executes arbitrary code in the proxy or the Hermes process.
- **Local File Disclosure:** A path that escapes the prefetch guard or the
  workspace-bounded read used by retrieval, exposing credentials or files
  outside the workspace.
- **Secret Exposure:** An API key, the management token, or the notify key
  appearing in logs, CCR entries, markers, error pages, or release artifacts.
- **Store Tampering or Integrity Break:** A way to overwrite, forge, or
  collide CCR entries so the agent retrieves content it did not compress, or a
  marker that resolves to attacker-chosen content.
- **Denial of Service (DoS):** Store growth, oversized bodies, a stalled hook
  callback, or a lock wedge that makes tool calls fail or blocks the agent.
- **Cross-Origin or DNS-Rebinding Abuse:** A web page reaching the loopback
  management routes to read stored session content or write CCR entries.
- **Authentication or Authorization Flaws:** A bypass of the loopback peer and
  Host check, or of the management bearer token when one is configured.
- **Supply Chain Issues:** A release asset that does not match its published
  checksum, a download path that bypasses SHA-256 verification, or a
  compromised build dependency.

### Where These Risks Live in the Tree&#x2001;🗺️

Each class above maps to code you can open. Naming that code makes a report
easier to route to the right maintainer.

**[`crates/aphrodite/src/prefetch.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/prefetch.rs)**

```rs
fn refuse_reason(path:&Path) -> Option<String> {
	// File-name denylist first - checked on the raw path so a relative
	// `.env` in the cwd is caught without any resolution.
	if let Some(name) = path.file_name().and_then(|n| n.to_str())
		&& (name == ".env" || name == "auth.json")
	{
		return Some(format!("refusing sensitive file name: {path:?}"));
	}
```

> [!NOTE]
>
> Prefetch refuses `.env` and `auth.json` by name, anything under `~/.ssh`,
> anything under the Hermes home, and non-file paths. A bypass of this guard is
> a local file disclosure finding, so include the exact path and whether it
> was reached through prefetch or retrieval.

**[`crates/aphrodite/src/main.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/main.rs)**

```rs
fn check_loopback_request(addr:SocketAddr, host_header:&str) -> Result<(), &'static str> {
	if !addr.ip().is_loopback() {
		return Err("only loopback clients allowed");
	}
```

> [!NOTE]
>
> Two layers protect the listeners: the peer address must be loopback, and the
> `Host` header must name one of `localhost`, `127.0.0.1`, `[::1]`, or `::1`.
> The second layer is the DNS-rebinding defense. `/health` is the one route
> exempt from the peer check.

**[`crates/aphrodite/src/main.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/main.rs)**

```rs
fn check_bearer_token(configured:Option<&str>, auth_header:&str) -> Result<(), &'static str> {
	let Some(token) = configured else {
		return Ok(());
	};
	if auth_header.strip_prefix("Bearer ") == Some(token) {
		Ok(())
	} else {
```

> [!NOTE]
>
> When `APHRODITE_MGMT_TOKEN` is set, the management routes require
> `Authorization: Bearer <token>`. When it is unset the routes accept any
> loopback caller, and startup logs a warning. `/metrics` is exempt from this
> token but not from the loopback check.

**[`crates/aphrodite-hermes/src/lib.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite-hermes/src/lib.rs)**

```rs
fn lock_bounded(state:&Arc<Mutex<AphroditeState>>) -> Option<std::sync::MutexGuard<'_, AphroditeState>> {
	let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
```

> [!NOTE]
>
> This is the 1.7.0 shared-gateway fix: each session owns its own state mutex,
> and a hook waits at most about 100 ms for it before degrading fail-open. A
> stall that still blocks tools after that bound is an availability bug worth a
> report, not a configuration problem.

**[`vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/vendor/headroom/crates/headroom-core/src/ccr/backends/sqlite.rs)**

```sql
DELETE FROM ccr_entries
WHERE last_accessed + ttl_seconds < ?1
   OR created_at + max_lifetime < ?2
```

> [!WARNING]
>
> That purge is the only bound on the token-mode store. There is no size cap,
> no LRU eviction, and no compaction for the SQLite tier, which is why the
> store can grow under sustained compression. See
> [Known Limitations](#known-limitations-and-availability-reports) before
> filing a store-growth report.

## Supported Versions&#x2001;🗂️

Aphrodite ships as a matched pair. The binary and the plugin must come from
the same release, and the version handshake fails closed on a mismatch.

| Component                       | Supported          | Notes                                                                                                                                       |
| :------------------------------ | :----------------- | :------------------------------------------------------------------------------------------------------------------------------------------ |
| Proxy binary `1.7.x`            | :white_check_mark: | Security fixes land on `Current` and ship in the next `1.7` patch release.                                                                  |
| Hermes plugin `2.3.x`           | :white_check_mark: | The plugin paired with binary `1.7.0`; install both halves from the same release.                                                           |
| Binary `1.6.x` / plugin `2.2.x` | :x:                | Superseded by the `1.7.0` shared-gateway lock fix. Upgrade before reporting a wedge or lockout.                                             |
| Binary `< 1.6.0`                | :x:                | Predates the per-session state registry, the bounded lock, and the fail-open degrade.                                                       |
| Unreleased `Current` branch     | :white_check_mark: | Actively developed; fixes are applied here first. If unsure which line you run, report against `Current` and include the commit you tested. |

Supported targets are `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`,
`x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`.

**[`.github/workflows/Build.yml`](https://github.com/PlayForm/Aphrodite/blob/Development/.github/workflows/Build.yml)**

```yaml
matrix:
    include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-gnu
        - os: macos-latest
          target: aarch64-apple-darwin
        - os: macos-latest # cross-compiled from arm64; macos-13 (real Intel host) queues indefinitely on GH-hosted runners
          target: x86_64-apple-darwin
        - os: windows-latest
          target: x86_64-pc-windows-msvc
```

> [!NOTE]
>
> Every target publishes `SHA256SUMS-<target>.txt`, and the setup path
> (`download.sh`, or `pwsh ./download.ps1` on Windows) verifies those sums
> before installing. `register()` does not download on the default path; the
> legacy `APHRODITE_AUTO_DOWNLOAD` opt-in is the one exception. A checksum
> mismatch is a supply chain finding: include the asset name, the expected sum,
> and the observed sum.

## Reporting a Security Vulnerability&#x2001;📮

**IMPORTANT: DO NOT report security vulnerabilities through public GitHub
issues, pull requests, or discussion forums for this repository.**

Public disclosure before a fix is available can put every Aphrodite user at
risk, and the store on a shared machine can hold other sessions' content.

**To report a security vulnerability, send a detailed email to:**

➡️ [Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud) ⬅️

Please include the following, as far as it applies to your finding.

### What Is Affected&#x2001;🏷️

1. **Subject Line:** Clearly mark the report and, if known, the component.
   Example: `Security Vulnerability Report: Aphrodite - [Brief Vulnerability Type]`

2. **Component:** The proxy binary, the Hermes dylib, the plugin shim, the
   release artifacts, or the repository tooling.

3. **Affected Version(s):** The binary version, the plugin version, and the
   commit hash or tag you tested. The two versions must both be current for the
   report to be prioritized.

4. **Vulnerability Type:** A classification such as RCE, local file disclosure,
   secret exposure, store tampering, DoS, cross-origin abuse, auth bypass, or
   supply chain.

### What Happens and How to Trigger It&#x2001;🧪

5. **Detailed Description:** What the issue is, which trust boundary it crosses,
   and why the current behavior is wrong.

6. **Steps to Reproduce (Proof of Concept):** Step-by-step instructions that
   reproduce the issue reliably. Include the config, the exact commands, the
   tool or HTTP call, and a minimal proof of concept where one is possible.

7. **Potential Impact:** What an attacker gains. For example: reading another
   session's stored content, extracting a provider key, corrupting the CCR
   store, or blocking tool calls until a restart.

8. **Environment Details:** Operating system and architecture, how Aphrodite is
   installed (plugin checkout, `cargo install`, or a release binary), the
   relevant `aphrodite.toml` settings with keys redacted, the runtime home, and
   the store path and size for availability reports. Gateway log lines are
   especially useful for hook stalls.

### How We Reach You Afterwards&#x2001;✉️

9. **Your Contact Information:** Your name or alias and an email address for
   follow-up. We may need to ask for a missing log line or a second run.

10. **Disclosure Intentions:** Any timeline you have in mind for public
    disclosure. We strongly prefer to coordinate a date after the fix ships.

11. **Credit/Acknowledgement:** Whether and how you want to be credited in the
    advisory.

### Reports Prepared With an AI Agent&#x2001;🤖

Reports drafted with the help of an AI agent are welcome. The human reporter
must have verified the observations on their own installation and must be able
to answer follow-up questions, because we triage the report with the person,
not the tool. Say which agent or tooling helped, as the report that prompted
this policy did, and keep the versions, logs, and reproductions attributable to
the reporter's own machine.

## Our Vulnerability Management Process&#x2001;⚙️

Upon receiving your private report at
[Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud), we follow this
process.

### Intake and Triage&#x2001;📥

1. **Acknowledgement (within 48 hours):** We confirm receipt and give you a
   tracking contact.

2. **Initial Triage and Validation (within 5 business days of
   acknowledgement):** A maintainer reproduces the issue against a supported
   version, assesses severity and scope, and may ask you for logs or a second
   run. If the report is against an unsupported version we will say so and ask
   you to retest on `1.7.x` before closing it. If we cannot reproduce it, or if
   it falls outside this policy, we explain the reasoning in writing and point
   you to the upstream project or the issue tracker where that is the better
   path.

### Investigation and Fix&#x2001;🔧

3. **Remediation Planning:** A confirmed issue is prioritized by severity. The
   fix may land in the proxy, the dylib, the plugin shim, the vendored store
   backend, or the release pipeline.

4. **Communication and Updates:** We send a status update at confirmation, when
   a fix is in review, when a release carrying it is published, and at least
   every 10 business days while the report is open. Questions in between are
   welcome.

5. **Fix Development and Testing:** The patch is reviewed and tested on the
   affected targets. A fix that changes the FFI contract or the store schema
   also updates the version handshake and the migration path.

### Coordinated Disclosure&#x2001;🤝

6. **Advisory and Release:** Once a fixed release is available we publish a
   GitHub Security Advisory, note the fix in the changelog, and credit the
   reporter if they asked to be credited. We ask for a 90-day remediation
   window before public disclosure by default; the window is negotiable when a
   fix takes longer or when the issue is already public.

| Phase           | Timeline         | Details                                                                                      |
| :-------------- | :--------------- | :------------------------------------------------------------------------------------------- |
| Acknowledgement | ≤48 h            | Confirmation of receipt and a tracking contact                                               |
| Triage          | ≤5 business days | Reproduction, severity, and impact scope                                                     |
| Fix             | ≤14 days         | Patch release for critical and high severity. Lower severity may queue for the next release. |
| Disclosure      | After the fix    | GitHub Security Advisory, changelog entry, and credit                                        |

If the issue is exploited in the wild, we expedite the whole sequence to
**≤72 hours** for the acknowledgement and initial mitigation guidance.

## Known Limitations and Availability Reports&#x2001;⚠️

Aphrodite is a local, single-tenant tool, and a few of its operational limits
are documented here rather than left for users to discover. They are still in
scope when they cause a denial of service or expose data, and a report with data
is welcome even when the underlying request is a feature.

### The token-mode store has no size cap&#x2001;🗄️

The token proxy persists CCR entries in SQLite at
`~/.hermes/aphrodite/ccr.db` by default, under `$APHRODITE_HOME` when that is
set, or at the path in `APHRODITE_DB`. Rows expire on a sliding idle TTL
(3600 seconds by default, `APHRODITE_CCR_TTL`) or on an absolute lifetime of
8x that TTL, and the purge runs lazily on read and write. Nothing bounds the
file by entry count or bytes.

| Tier                 | Backend                | Bound                   | Expiry                            |
| :------------------- | :--------------------- | :---------------------- | :-------------------------------- |
| Inline (proxy)       | `lru::LruCache`        | 1,024 entries           | LRU eviction only                 |
| Inline (session)     | HashMap plus LRU order | 500 entries and 256 MiB | LRU and byte-budget eviction      |
| Cache mode (`:9797`) | In-memory store        | 10,000 entries          | Sliding idle TTL, 8x max lifetime |
| Token mode (`:9798`) | SQLite `ccr.db`        | **none**                | Sliding idle TTL, 8x max lifetime |

Under sustained compression the database can grow to gigabytes, and a full
disk then surfaces as proxy write failures and slower hooks. The operator reset
is to stop the proxy, delete `ccr.db`, `ccr.db-wal`, and `ccr.db-shm`, and
relaunch. Every marker that pointed into that store becomes unresolvable, which
is expected.

Size-capped retention (`store_max_entries` / `store_max_bytes` with LRU
eviction or fill-triggered compaction) and per-session store isolation are not
available in 1.7.0. They are tracked as feature requests. A report that shows
the failure mode with a fill curve, a session count, and the store size is
still triaged as an availability bug rather than closed as a feature request.

### `fill_pct` measures compression ratio, not store occupancy&#x2001;📉

`fill_pct`, the value behind `X-Aphrodite-Fill-Pct` and the `fill_pct` metric,
is derived from the compression-ratio moving average:
`100 - ratio_ema / 20`, clamped to `[1, 99]`
([`compute_fill_pct`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/proxy.rs)).
A fresh process reports `90.0` because the seed ratio is `2.0x`, so a high
`fill_pct` means the engine sees little headroom, not that the disk is 90%
full. When you report store growth, measure the `ccr.db` file and its WAL
sidecar; a report that quotes `fill_pct` as occupancy will be re-checked
against the file size.

### `pre_tool_call` is the only fail-closed hook&#x2001;🔒

Hermes treats `pre_tool_call` as fail-closed: a callback that is still running
when the dispatch timeout fires blocks the tool with
`pre_tool_call plugin callback timed out or is still running`, and suppresses
that callback for 60 seconds. Every other Aphrodite hook fails open.

The 1.7.0 fix bounds lock acquisition at about 100 ms and degrades fail-open,
but the bound covers the session state lock, not every code path in the
callback. A callback stalled elsewhere, or a store and disk slow enough to
exceed the Hermes dispatch timeout, can still block tool calls until the
gateway process restarts. If you hit this on 1.7.0, send the abandoned-worker
log lines, the hook durations, the `ccr.db` and WAL sizes, the session count,
and your `aphrodite.toml`.

### Management routes are unauthenticated by default&#x2001;🔑

`APHRODITE_MGMT_TOKEN` is unset by default. The management routes then accept
any loopback caller, protected only by the peer and Host checks, and startup
logs a warning. Set the token on a shared machine. The proxy has no CORS layer
by design, because no browser client needs one, so a page in a local browser
cannot read management responses; the bearer token closes the write side that a
CORS-simple request could still reach.

### The token store is plaintext at rest&#x2001;💾

`ccr.db` stores prompt content and tool output as plain SQLite rows, and the
stored content stays readable to anyone who can read the file. There is no
at-rest encryption inside Aphrodite. Use full-disk encryption and restrict the
runtime home's permissions on multi-user machines. Cache mode keeps entries in
memory only and loses them on restart.

## Confidentiality&#x2001;🔒

We treat your report and our correspondence as confidential. We do not share
the details with third parties without your permission, except as needed to
investigate and fix the issue, for example with the maintainers of an affected
upstream dependency or with the Hermes maintainers when the fix crosses that
boundary.

## Scope Exclusions (What Not to Report Here)&#x2001;🚫

- **Denial of service against public infrastructure:** Do not run load tests
  against PlayForm websites, release infrastructure, or build runners.
- **Social engineering:** Reports of phishing or impersonation of project
  members are handled outside this code policy.
- **Missing best practices without impact:** A hardening suggestion with no
  demonstrable effect belongs in a regular issue.
- **Unvalidated scanner output:** A scanner finding is a starting point, not a
  report. Validate it and include a reproduction.
- **Self-XSS:** An issue that requires the user to paste code into their own
  console or modify their own machine is not a vulnerability here.
- **Physical access:** An issue that requires an unlocked machine and a local
  shell is out of scope, though the store's at-rest exposure is documented
  above.
- **Feature requests:** Retention knobs and per-session store isolation are
  tracked as features. Report them as security only when you can show the
  denial of service or the cross-session exposure they cause.
- **Hermes Agent, the model provider, and the operating system:** Report those
  upstream, and copy us when Aphrodite is the vector.

When in doubt, err on the side of reporting to
[Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud).

## Questions & Contact&#x2001;💬

For questions about this policy or the reporting process, contact
[Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud). For ordinary bugs,
feature requests, and usage questions, use the
[issue tracker](https://github.com/PlayForm/Aphrodite/issues).

Thank you for helping make Aphrodite safer for everyone who runs an agent on
their own machine.
