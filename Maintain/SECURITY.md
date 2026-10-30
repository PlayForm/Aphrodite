# Maintainer Security Handbook&#x2001;🛠️

> [!NOTE]
>
> This is the maintainer-side counterpart to the repository's public
> [`SECURITY.md`](../SECURITY.md). The root file is the contract we publish to
> users and researchers; this file is the operating manual for maintainers and
> release owners. It describes conditions that are not fixed yet, so do not
> paste it into a public issue or quote it as a commitment.

## Relationship to the Public Policy&#x2001;🔗

The root policy owns the audience, the reporting address, the supported
versions, the response timelines, the known limitations, and the exclusions.
This handbook owns how we meet those commitments: the threat model we maintain,
the controls that exist in the code, the triage runbook, and the remediation
backlog.

When the security posture changes, update both files in the same change. In
particular, the root file's Known Limitations section and the backlog below
must not disagree, because a reporter will read the public file and expect the
internal one to match it.

## Current Posture&#x2001;📋

The state below is the `1.7.0` binary with the `2.3.0` plugin.

| Area                          | State                                                                                                 | Maintainer action                                                                                   |
| :---------------------------- | :---------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------- |
| Shared-gateway lock wedge     | Fixed in `1.7.0`: per-session state registry, a bounded (~100 ms) lock, and a fail-open degrade       | Keep the regression probes in `crates/aphrodite-hermes/src/lib.rs` green.                           |
| Token-mode store size         | Unbounded by bytes and entries; only the TTL purge limits it                                          | Retention design is open; document the operator reset until it ships.                               |
| Per-session store isolation   | Not implemented; every session compresses into one `ccr.db`                                           | A reset invalidates markers for all sessions. Tracked as a feature request.                         |
| Management route auth         | `APHRODITE_MGMT_TOKEN` unset by default; loopback peer and Host checks only                            | Startup already warns; hardening is open.                                                           |
| At-rest encryption            | None; `ccr.db` is plaintext SQLite                                                                    | Full-disk encryption guidance only.                                                                 |
| FFI dylib                     | Loaded into the Hermes process; the version handshake fails closed                                     | Treat the dylib as trusted code in the agent process; review every FFI change as a security change. |
| CORS                          | No CORS layer by design; the proxy has no browser consumer                                             | Do not add `CorsLayer::permissive()`; it was removed for a reason (report 12 F1).                   |

## Threat Model&#x2001;🧠

Aphrodite is a local, single-tenant compression proxy for Hermes Agent. It
binds loopback, rejects non-loopback peers, and stores compressed content on
the user's machine. The assets worth protecting are prompt content, tool
output, provider credentials, the management token, the store's integrity, and
the agent's ability to keep working.

| Threat                          | Impact            | Mitigation                                                                                                             |
| :------------------------------ | :---------------- | :--------------------------------------------------------------------------------------------------------------------- |
| Prompt and tool-output exposure | High              | Cache mode keeps entries in memory; token mode stores plaintext in `ccr.db`, so full-disk encryption and file permissions carry the risk. |
| Provider key theft              | High              | The key comes from `aphrodite.toml` or the environment, is sent only to the provider, and is never written into a CCR entry or marker. Treat a config file that carries a key as a secret. |
| CCR store tampering             | Medium            | Local file with WAL; no network-accessible SQL surface; management writes are gated by the optional bearer token and the loopback checks. |
| Denial of service via growth    | High availability | No size cap on the token store; TTL purge only. Operator reset exists; retention is open.                              |
| Hook stall and lock wedge       | High availability | Per-session state, bounded lock acquisition, fail-open degrade. `pre_tool_call` remains fail-closed in Hermes dispatch. |
| Cross-origin management abuse   | Medium            | No CORS layer; loopback peer and Host allowlist; optional bearer token.                                                |
| Supply chain compromise         | High              | `Cargo.lock`, `deny.toml`, `cargo deny` in CI, SHA-256 sums per target, explicit setup instead of download at register. |

### Trust Boundaries&#x2001;🧱

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   Hermes Agent   │────▶│  Aphrodite Proxy │────▶│   LLM Provider   │
└──────────────────┘     └────────┬─────────┘     └──────────────────┘
                                  │
                         ┌────────┴─────────┐
                         │                   │
                   ┌─────▼─────┐     ┌──────▼──────┐
                   │ Cache CCR │     │  Token CCR  │
                   │ (memory)  │     │  (SQLite)   │
                   └───────────┘     └─────────────┘
```

- **Hermes Agent to Aphrodite:** loopback only, enforced by the peer address
  and the `Host` allowlist. No inbound authentication by default.
- **Aphrodite to the provider:** TLS through rustls, no system OpenSSL.
- **CCR stores:** in memory in cache mode, SQLite in token mode. No remote
  access, no network SQL interface.
- **The dylib:** runs inside the Hermes process with the agent's privileges.
  A dylib loaded from a writable path is code execution in that process, which
  is why the runtime home's permissions and the binary resolution order matter.

## Controls in the Code&#x2001;🛡️

| Control                        | Location                                                                                                              | Notes                                                                                                       |
| :----------------------------- | :-------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------- |
| Loopback peer and Host check   | `check_loopback_request` in [`crates/aphrodite/src/main.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/main.rs) | Rejects non-loopback peers and non-loopback `Host` values; `/health` is exempt from the peer check.          |
| Management bearer token        | `require_mgmt_token` in the same file                                                                                 | Applied to management routes only, never to the catch-all LLM route; `/metrics` is exempt from the token.    |
| Body limits                    | Router layers in the same file                                                                                        | 1 MiB on management routes, 64 MiB on the catch-all route, plus the optional `max_request_body_size` cap.    |
| Prefetch refusal               | `refuse_reason` in [`crates/aphrodite/src/prefetch.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/prefetch.rs) | Refuses `.env` and `auth.json` by name, `~/.ssh`, the Hermes home, and non-file paths.                      |
| Workspace-bounded file reads   | `read_path_guarded` in [`crates/aphrodite-hermes/src/tools.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite-hermes/src/tools.rs) | Canonicalizes the path, requires it under the current workspace, and caps the read at 10 MiB.                |
| Prefetch size cap              | `MAX_PREFETCH_SIZE` in `prefetch.rs`                                                                                  | 10 MiB per file.                                                                                            |
| Content addressing             | BLAKE3 hashes, 40 hex characters                                                                                      | A marker is only emitted when the store actually holds the hash; a failed store leaves content uncompressed. |
| Store eviction                 | [`crates/aphrodite/src/state/store.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite/src/state/store.rs) | Inline session store: 500 entries and a 256 MiB budget. Inline proxy cache: 1,024 LRU entries.              |
| Bounded session lock           | `lock_bounded` in [`crates/aphrodite-hermes/src/lib.rs`](https://github.com/PlayForm/Aphrodite/blob/Development/crates/aphrodite-hermes/src/lib.rs) | About 100 ms, then fail-open. The registry mutex is held only for the get-or-create.                        |
| FFI version handshake          | Plugin `_check_version` against the dylib                                                                             | A mismatch fails closed; never report results from a mismatched dylib.                                      |
| No CORS layer                  | Router construction in `main.rs`                                                                                      | Deliberate; see the report 12 F1 note in the code.                                                          |

## Data Protection&#x2001;💾

| Data              | At rest                                                              | In transit          |
| :---------------- | :------------------------------------------------------------------- | :------------------ |
| Prompts           | Plaintext SQLite in token mode; memory only in cache mode            | TLS to the provider |
| Responses         | Same as prompts                                                      | TLS to the provider |
| Provider key      | `aphrodite.toml` or the environment; never inside a CCR entry        | TLS to the provider |
| Management token  | Environment only                                                     | Loopback            |
| Notify key        | Environment or CLI; sent as a bearer header to `notify_url`          | Whatever `notify_url` uses |
| CCR payloads      | Plaintext in `ccr.db` and its WAL sidecar                            | Loopback            |
| Metrics and stats | Plaintext, loopback only                                             | Loopback            |

The env-gated debug logger (`APHRODITE_DEBUG`) can emit request and marker
detail. Treat its output as sensitive and never attach it unredacted to a
public issue.

## Secrets Management&#x2001;🔑

- **Provider key:** resolution order is `proxy.api_key`, then
  `defaults.api_key`, then `APHRODITE_API_KEY`. The TOML value wins, which is
  why a config file with a key in it is a secret file.
- **Management token:** `APHRODITE_MGMT_TOKEN` only.
- **Notify key:** `APHRODITE_NOTIFY_KEY` or the matching CLI flag; sent as a
  bearer header to the operator-configured `notify_url`.
- **Upstream forwarding:** the provider key is set as the `Authorization`
  bearer header on the outbound request; it is not copied into CCR entries,
  markers, stats, or history.
- **Reports:** redact every key, token, and `Authorization` header before a
  config or log excerpt goes into an advisory or a commit message.

## Supply Chain&#x2001;📦

### Dependencies

- `Cargo.lock` is committed, and the workspace `Cargo.toml` is the single
  source of version pinning.
- `deny.toml` carries the advisory ignore list with a written rationale for
  each entry, plus the license allowlist. The current ignores are unmaintained
  transitive crates with no safe upgrade, not accepted vulnerabilities.
- `cargo deny check` runs in CI (`.github/workflows/Check.yml`). A red result
  is a policy decision, not something to silence: triage the finding, then
  either fix it or record the exception in `deny.toml` with a reason.
- The runtime is Rust only. npm is dev tooling (Prettier), and Python is the
  plugin shim and its tests; neither ships inside the binary.

### Update Automation

| Ecosystem      | Schedule | Notes                                      |
| :------------- | :------- | :----------------------------------------- |
| github-actions | daily    | Workflow actions are pinned by commit too. |
| npm            | daily    | Dev tooling only.                          |
| cargo          | weekly   | The runtime dependency tree.               |
| gitsubmodule   | weekly   | Vendored and child repositories.           |

### Release Artifacts

- `Build.yml` builds four targets and stages `aphrodite-<target>`,
  `libaphrodite_hermes-<target>`, and `SHA256SUMS-<target>.txt` for each.
- The `Finalize` job asserts that every target's assets and checksums exist
  before the release is considered complete. Do not publish notes that claim a
  full platform set while that job is failing.
- The child plugin repository carries an in-tree `SHA256SUMS.txt` generated by
  the same workflow; the setup path (`download.sh` / `download.ps1`) verifies
  the sums before installing, and `register()` does not download on the default
  path.
- Verify a release by checking that the tag points at the commit whose
  `Cargo.toml` reports the release version, then compare the downloaded assets
  against `SHA256SUMS.txt`.

## Triage Runbook&#x2001;🧭

### 1. Intake&#x2001;📥

- Acknowledge within 48 hours from [Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud)
  and name the maintainer who owns the report.
- Open a draft GitHub Security Advisory immediately, so the discussion stays
  private and the eventual publication path is ready.
- Record the reporter, the component, the versions, and the claimed class in
  the advisory draft. Do not move the discussion to a public issue.

### 2. Severity&#x2001;🎚️

| Severity | Examples                                                                                                                       | Response                                    |
| :------- | :----------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------ |
| Critical | RCE in the proxy, dylib, or shim; release artifact substitution; key exfiltration with no local precondition                     | Fix and release within 14 days; expedite to 72 hours when exploited in the wild. |
| High     | Local file disclosure outside the workspace; management auth bypass when a token is set; store integrity break; default-config lockout | Fix within 14 days.                         |
| Medium   | DoS that needs an unusual load or a local attacker; metadata leak without secrets; hardening gap with a demonstrated effect     | Next release cycle.                         |
| Low      | Defense in depth; missing best practice without exploitable impact                                                             | Convert to a regular issue unless the reporter wants an advisory. |

### 3. Reproduction&#x2001;🧪

- Confirm the binary version, the plugin version, the commit, the OS, and the
  architecture. A report against `1.6.x` or an older line is closed as
  superseded unless the same reproduction holds on `1.7.x`.
- Reproduce on the reporter's target first, then on a second target when the
  fix touches the FFI or the store.
- Capture the config with keys redacted, the runtime home, the store path, and
  the exact commands. For hook issues, capture the gateway log lines and the
  dylib version handshake result.

### 4. Availability and Store Growth&#x2001;🗄️

This is the playbook for a report like the multi-session gateway report that
prompted the public policy.

1. Locate the store: `~/.hermes/aphrodite/ccr.db` by default, under
   `$APHRODITE_HOME` when set, or at `APHRODITE_DB`. Record
   `ls -lh ccr.db ccr.db-wal ccr.db-shm` and the filesystem's free space.
2. Do not accept `fill_pct` as occupancy. It is `100 - ratio_ema / 20` from
   the compression-ratio EMA, and a fresh process starts at `90.0`. Compare
   the reported value with the file size; a report that confuses the two is
   usually a compression-ratio complaint, not a full disk.
3. Count the sessions and confirm the shared-gateway model. The dylib serves
   every session in one process, and the token store is shared across all of
   them, so aggregate write pressure scales with session count.
4. Reproduce under sustained compression with large tool output, sampling the
   store size over time. Note the idle TTL (3600 s by default) and the 8x
   absolute lifetime; rows do expire, but nothing caps the file.
5. Classify: a lockout or data loss is a High availability issue; slow
   compression with a large store is a bug, not a vulnerability.
6. Workaround for the reporter: stop the proxy, delete `ccr.db`,
   `ccr.db-wal`, and `ccr.db-shm`, and relaunch. Warn that every marker that
   pointed into the store becomes unresolvable.
7. Fix path: add retention to the SQLite backend (entry and byte caps, LRU or
   fill-triggered compaction), plumb it through `aphrodite.toml` and the
   `APHRODITE_*` surface, and cover it with a migration test. Per-session
   isolation is a larger change: a session-scoped database path, a migration
   for existing stores, and a decision about what happens to markers when a
   session ends.

### 5. Hook Stall and Lockout&#x2001;🔒

1. The signature is `pre_tool_call plugin callback timed out or is still
   running`, often with abandoned-worker lines in the gateway log.
2. Confirm the dylib version and the handshake. A `1.6.x` dylib predates the
   per-session registry and must be upgraded before triage continues.
3. On `1.7.x`, verify the session ids are distinct and that the hook returns
   within the bounded lock window. The fail-open probe
   (`probe_wedged_session_lock_degrades_fail_open`) is the reference behavior.
4. Remember the Hermes side: `pre_tool_call` is the only fail-closed hook,
   the dispatch timeout is Hermes-owned, and a timed-out callback is suppressed
   for 60 seconds. A slow callback therefore costs more than one tool call.
5. Collect hook durations, the session count, the store size, the WAL size,
   the config, and the exact log lines. If the stall reproduces on `1.7.x`,
   treat it as an availability regression and add a probe before fixing.

### 6. Fix, Release, and Advisory&#x2001;🚀

- Fix on `Current`, add a regression test, and keep the change scoped to the
  reported class.
- Bump the binary version and the paired plugin version, update the changelog
  under a security heading, and let `Build.yml` produce and verify the
  artifacts. Do not tag before `Finalize` passes.
- Publish the GitHub Security Advisory with affected versions, patched
  versions, the workaround, and the credit line the reporter agreed to.
- Request a CVE only when the issue is independently useful to the wider
  ecosystem; the advisory is the primary record.
- Backport to the latest `1.7.x` only. Older lines are end-of-life and stay
  unsupported.

### 7. Post-Incident&#x2001;📝

- If the condition remains after the fix, add or update the Known Limitations
  section in the root policy.
- Update the backlog below and the posture table above.
- If the FFI contract or the store schema changed, bump the version handshake
  and add the migration test.
- If the fix depends on Hermes dispatch behavior, coordinate with the Hermes
  maintainers before the advisory publishes.

## Open Remediation Backlog&#x2001;📌

| Item                                              | Why it matters                                                                 | State                     |
| :------------------------------------------------ | :----------------------------------------------------------------------------- | :------------------------ |
| Size-capped retention (`store_max_entries`, `store_max_bytes`, LRU, compaction) | The token store has no byte or entry bound; a full disk degrades the proxy and slows hooks. | Open; not in `1.7.0`.     |
| Per-session store isolation                       | One shared `ccr.db` lets one session's reset or eviction invalidate another session's markers. | Open; not in `1.7.0`.     |
| Management auth hardening                         | The management routes accept any loopback caller when the token is unset.       | Open; startup warning only. |
| At-rest encryption option                         | `ccr.db` is plaintext; full-disk encryption is the only current answer.         | Open.                     |
| Documented reset command                          | Operators delete `ccr.db`, the WAL, and the SHM by hand.                        | Open; described in the policy. |
| Store-occupancy telemetry                         | `fill_pct` measures compression ratio, not bytes, which invites misreports.     | Open.                     |

## Internal Contacts and Roles&#x2001;📇

- **Security inbox:** [Security@PlayForm.Cloud](mailto:Security@PlayForm.Cloud).
  The shared inbox is the system of record for every report.
- **Report owner:** the maintainer who acknowledges the report owns it until
  the advisory is published or the report is declined.
- **Release owner:** the maintainer who cuts the patch release and verifies
  the checksums and the `Finalize` job.
- **Hermes coordination:** the Hermes maintainers, when a fix depends on the
  hook dispatch contract rather than on Aphrodite alone.
- **Disclosure discipline:** the draft advisory is the only place a report is
  discussed. Do not open a public issue, a pull request, or a chat thread for
  an open report.

## Version History&#x2001;🗓️

| Version | Date       | Changes                                                                                                                                          |
| :------ | :--------- | :----------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.0.0   | 2026-06-16 | Initial security document for Aphrodite.                                                                                                         |
| 2.0.0   | 2026-10-30 | Split the public policy into the root `SECURITY.md`; refreshed versions and controls for `1.7.x` / `2.3.x`; added the known limitations, the triage runbook, and the remediation backlog. |
