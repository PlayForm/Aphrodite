# Aphrodite Governance - VERIFICATION-MATRIX.md

Global claims -> read-only probe -> expected output -> failure action. Every
risky operational claim in the Aphrodite skill system resolves to a row here
or in a skill's own local test matrix. A claim with no probe is not an
operational instruction; it is a hypothesis.

## Global verification matrix

| Claim                                                                   | Read-only probe                                                                                                                                                                                                                                                            | Expected output                                                                                                       | Failure action                                                                   |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| Git orientation state (root, branch, clean)                             | `git rev-parse --show-toplevel && git branch --show-current && git status --short`                                                                                                                                                                                         | Expected repo root, permitted branch, empty status                                                                    | Stop; run `aphrodite-orientation` interpretation rules                           |
| Auto-committer baseline captured                                        | `git rev-parse HEAD; git ls-remote origin <branch>`                                                                                                                                                                                                                        | HEAD + remote tip recorded before first mutation                                                                      | Re-run gate; do not proceed without baseline                                     |
| Submodule state valid                                                   | `git submodule status --recursive`                                                                                                                                                                                                                                         | No `+`, no `-`, no unexpected mode-160000 entry                                                                       | Stop; submodule diagnosis in `aphrodite-orientation`                             |
| Engine loaded (registration + config selection + controlled round trip) | 1. Plugin load confirmed 2. Engine instance subclasses Hermes `ContextEngine` 3. Registration accepted 4. Aphrodite selected as active engine 5. Hermes built-in compression disabled 6. `aphrodite_test(mode="quick")` round trip 7. exactly one active compression owner | All seven conditions true; round trip `status="ok"`                                                                   | Enter **hooks-only** mode; never report "partially active context engine"        |
| Engine loaded - NOT just an env var                                     | Env var presence alone                                                                                                                                                                                                                                                     | (No claim may be based on this alone)                                                                                 | Re-run the seven-condition check above                                           |
| Proxy healthy - local engine availability                               | Local health endpoint (no upstream access required)                                                                                                                                                                                                                        | Local endpoint succeeds; cache/store accepts and returns a known payload; marker resolves locally                     | Remediate local engine (restart, dylib, config) before touching upstream         |
| Proxy healthy - upstream API reachability (SEPARATE probe)              | Upstream health endpoint                                                                                                                                                                                                                                                   | Upstream reachable; if not, degraded mode is explicitly supported (e.g. inline-only compression)                      | Never assume recovery; document the degraded mode                                |
| Release artifact exists                                                 | Exact consumer download names, platform naming, checksum behavior                                                                                                                                                                                                          | Named assets present for macOS/Windows/Linux consumer paths; optional assets absent degrade with warning, not failure | Do not bump `BINARYVERSION`; fix artifact build/attach first                     |
| Version bump correct                                                    | Version ledger (below) - every owned location checked                                                                                                                                                                                                                      | Each row's value matches its authority and earliest/latest-safe windows                                               | Stop; fix the out-of-window value before any release claim                       |
| Plugin code changed (fresh-process/reload test)                         | Restart the plugin process / hot-reload, then read loaded dylib version and run a behavior probe                                                                                                                                                                           | New version and new behavior observed in a **fresh** process                                                          | Results may be from a stale dylib/symlink; restart and re-test before concluding |
| Release tag has intended effects                                        | Workflow files at the exact tag commit (trigger audit)                                                                                                                                                                                                                     | Only accepted jobs reachable from the tag                                                                             | Change the workflow or halt the tag                                              |
| CCR marker retrievable                                                  | `aphrodite_retrieve(hash=...)` on a known compressed payload                                                                                                                                                                                                               | Normalized source equals result                                                                                       | Stop release/debug implementation; check resolver                                |
| Retrieval is non-recompressible                                         | Feed a retrieval response back through the transform                                                                                                                                                                                                                       | Payload stays raw; no nested marker                                                                                   | Update the skip classifier (owner: compression-safety)                           |
| Secrets reach the child process safely                                  | Health/start test with redacted key presence                                                                                                                                                                                                                               | Proxy starts without key error; no secret printed                                                                     | Fix passthrough/config; never echo the secret                                    |

## Version ledger (canonical, 5 rows)

The three version values (binary release version, plugin package version,
`BINARYVERSION` compatibility/download pointer) are never described as one
generic "version bump." Authority, earliest update, latest safe update, and
verification are per row:

| Field                | Meaning                                | Authority            | Earliest update                            | Latest safe update                | Verification                                 |
| -------------------- | -------------------------------------- | -------------------- | ------------------------------------------ | --------------------------------- | -------------------------------------------- |
| Binary version       | Published Rust artifact identity       | Parent release owner | Development prep                           | Release-sync commit               | Binary `--version`, manifests, artifact name |
| Plugin version       | Plugin package identity                | Plugin owner         | Plugin prep                                | Plugin Current release commit     | `plugin.yaml`, install metadata              |
| `BINARYVERSION`      | Binary expected by plugin installer    | Plugin release owner | Local prep only if artifact already exists | After artifact/tag availability   | Consumer download resolution                 |
| Parent gitlink       | Exact plugin commit consumed by parent | Parent Current owner | Parent sync                                | Before parent release tag         | Submodule status/tree entry                  |
| README badge/example | Documentation claim                    | Documentation owner  | After authoritative value changes          | Before release notes finalization | Render/source scan                           |

The "local bump can be safe, public distribution pointer must not be early"
distinction is explicit: a local `BINARYVERSION` bump ahead of the tag is safe
when the binaries already exist locally; the "bump LAST" rule applies at tag
time.

## Claim-to-test matrix policy

Every active skill must **end with its own local test matrix** using this
schema - the global matrix covers shared claims, the skill matrix covers
skill-specific claims:

| Claim                  | Evidence source | Test           | Pass condition       | Failure response       |
| ---------------------- | --------------- | -------------- | -------------------- | ---------------------- |
| (skill-specific claim) | (named source)  | (bounded test) | (observable success) | (named failure action) |

Core rule: **if a claim cannot be tested, do not write an operational
instruction that relies on it.** The manifest's "Source of truth" column for
each skill must name a file or executable probe; the skill's local test matrix
must end with the same probes.

## Matrix maintenance

- Add a row when a skill introduces a new risky claim; never bury the probe in
  prose only.
- When a probe changes (source moves, tool renamed), update the matrix row and
  the owning skill in the same change; log the drift in
  CONTRADICTION-REGISTER.md.
- Threshold literals never appear in an expected output unless the step first
  reads the active configuration and prints the value it is about to test
  (live-read rule).
