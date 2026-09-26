---
name: aphrodite-cargo-upgrade
description: "Use when upgrading cargo deps in PlayForm/Aphrodite (crates/aphrodite + vendor/headroom owned fork). Decision tree: inventory, one compatibility cluster at a time, minimal-target compile, failure classification, migrate/pin/revert/defer, runtime behavioral tests, pin lifecycle."
version: 1.4.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-cargo-upgrade
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, cargo, upgrade, rust, breakpoints, pinning, migrate, compile]
        related_skills: [aphrodite-boundaries, aphrodite-orientation]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
    runtime_modes:
        - source
owns:
    - dependency upgrade and pinning decisions for the workspace, crates/aphrodite, and vendor/headroom
    - classification of upgrade failures (API rename, trait-bound, feature rename, semver, runtime route, ABI/FFI, security advisory)
    - the compatibility-pin ledger, its removal conditions, and its review deadlines
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
supersedes: []
verification:
    source_of_truth:
        - workspace and crate Cargo.toml manifests, Cargo.lock
        - references/breakpoints.md (known breakpoints, manifest-verified)
        - runtime probes: aphrodite --version, route/WS/FFI/hash round trips
mutation_level: local
---

# Aphrodite Cargo Upgrade (Decision Tree)

Canonical procedure for dependency upgrades in PlayForm/Aphrodite: parent
workspace (`crates/aphrodite`, `crates/aphrodite-hermes`) and the
`vendor/headroom` OWNED fork, a submodule that may be modified freely
(edition-2024 workspace, declared in its `Cargo.toml`). Historical entry
point: the house-style Cargo update wrapper script (environment-specific;
CLAIM, not probed here). Its `ExpandVersions` helper hits the toml_edit
breakpoint recorded in `references/breakpoints.md` (read that file to
confirm).

Anti-pattern this tree prevents: pin everything until it compiles. Pinning
until it compiles is not an upgrade; the graph can compile with a split
dependency set while a runtime break goes unseen. Probes: `cargo tree -p
headroom-proxy` (split); Step 6 runtime list (breakage).

## Preflight (delegated - do not duplicate)

Run the **aphrodite-orientation** gate first (repository root, permitted
branch `Development`, clean/known worktree, submodule status, auto-committer
HEAD snapshot), then apply **aphrodite-boundaries** stop/recovery rules at
every step. `mutation_level: local`: no commit/tag/push here; the
auto-committer sweeps (orientation's HEAD snapshot is the test).

- Never mutate before a precondition is verified, because the failure
  becomes unattributable to the change.
- Never continue after a failed verification, because the baseline is then
  stale and every later decision inherits it.
- Never use `git checkout`/`reset` to repair content, because it discards
  working-tree changes the Step 1 diff must record.
- Never commit or push from this skill, because the auto-committer owns the
  sweep and a manual commit races it.

## Step 1 - Inventory the upgrade surface

Capture the exact pre-upgrade state; every later decision needs this
baseline. Preconditions: orientation gate passed; no unrelated manifest or
Cargo.lock changes.

**Do**

```sh
git diff HEAD -- Cargo.lock 'crates/*/Cargo.toml' 'vendor/headroom/Cargo.toml' 'vendor/headroom/crates/*/Cargo.toml'
cargo tree --workspace --edges normal
rustc --version
cargo --version
find . -name Cargo.toml -not -path '*/target/*' -not -path './plugins/*'
```

**Verify**

```sh
git status --short -- Cargo.lock 'crates/*/Cargo.toml' 'vendor/headroom/**/Cargo.toml'
echo "EXIT:$?"
```

Recorded: lockfile diff, package graph, toolchain version, full manifest
list, no uncommitted manifest edits.

**Stop if** the lockfile or a manifest differs from what the diff shows
(auto-committer race). Re-capture; do not proceed on stale evidence.

**Recovery**: permitted - re-run the read-only inventory commands and edit
files directly; prohibited - `git checkout`/`git restore`/`git reset` to
"clean" the diff, because cleaning erases the baseline record the diff is.

## Step 2 - Upgrade one compatibility cluster at a time

Bound blast radius so a failure is attributable to one co-changing group.
Precondition: Step 1 baseline recorded.

**Do**

```sh
cargo upgrade --dry-run <cluster>   # e.g. axum, or tokio-tungstenite alone
# then, for the accepted cluster only:
cargo update -p <cluster-member> --dry-run
```

The proposed change must touch ONE compatibility cluster: networking
(reqwest/tokio-tungstenite), web routing (axum), Python FFI
(pyo3/pyo3-log), crypto/hash (sha2), or an unrelated single crate.

**Stop if** the upgrade moves unrelated networking + FFI + crypto + web deps
together. That requires an explicit compatibility matrix (each crate
compiled AND runtime-tested at the proposed versions) before it is allowed.

**Recovery**: permitted - split into per-cluster `cargo upgrade` runs;
prohibited - pinning the whole graph until it compiles, because that
re-creates the split graph the anti-pattern names.

## Step 3 - Compile minimal targets independently

Prove each layer compiles before any runtime claim. Precondition: Step
2 accepted exactly one cluster.

**Do**

```sh
cargo check -p aphrodite
cargo check -p aphrodite-hermes
cargo check --manifest-path vendor/headroom/Cargo.toml                   # default-members only (skips headroom-py)
cargo check --manifest-path vendor/headroom/Cargo.toml -p headroom-proxy # WS/axum crate
cargo check --manifest-path vendor/headroom/Cargo.toml -p headroom-py    # FFI boundary
cargo check --manifest-path vendor/rtk/Cargo.toml                        # vendored submodule
```

**Verify**

```sh
for t in aphrodite aphrodite-hermes headroom-proxy headroom-py; do echo "$t: $(
	test -z "$(cargo check -p $t 2>&1 > /dev/null)"
	echo $?
)"; done
echo "EXIT:$?"
```

**Stop if** a failure's cause cannot be assigned to the Step 2 cluster.

**Recovery**: permitted - proceed to Step 4 with the isolated failure;
prohibited - "fixing" a different cluster's manifest to make this one pass,
because that moves the failure outside the blast radius.

## Step 4 - Classify the failure

Map the symptom to ONE failure class; each class has one allowed action.
Precondition: Step 3 produced a compile error or a known runtime suspicion.

Classes (recognition strings and examples in
`references/runtime-tests.md`; default actions): API rename → migrate
source; trait-bound change → migrate source or pin locally; feature rename
→ migrate feature string; semver incompatibility → pin locally or revert;
runtime route behavior → pin + behavioral tests; ABI/FFI change → pin +
FFI contract gates; security advisory → pin to patched minor or defer.

**Stop if** two classes both plausibly match. Resolve by compiling the crate
at the old version to isolate the change.

**Recovery**: permitted - `cargo tree -p <dep>` and `cargo update --dry-run`
to inspect; prohibited - guessing from stale skill prose, because the
checked-out manifest is the source-derived fact; re-derive from it.

## Step 5 - Choose the action

Apply the Step 4 default action; a pin gets its ledger entry immediately
(template below). Precondition: Step 4 classification recorded. Action
precedence: migrate source > pin locally > pin workspace-wide > revert >
defer.

**Verify**

```sh
grep -n '^<dep> *=' crates/*/Cargo.toml vendor/headroom/Cargo.toml vendor/headroom/crates/*/Cargo.toml
echo "EXIT:$?"
```

**Stop if** a pin would be added without a ledger entry. Unexplained pins
become permanent debt and hide split graphs.

**Recovery**: permitted - edit the manifest and ledger directly, rerun Step
3; prohibited - pinning "temporarily" without expiry/revisit, because a pin
without a removal condition is ungoverned debt.

### Compatibility pin

**Dependency:** `pyo3`
**Pinned version:** `<exact version>`
**Reason:** Required API was removed upstream; migration not yet implemented.
**Affected surface:** Python callback/thread boundary.
**Evidence:** Failing migration compile/test case.
**Owner:** `<team or area>`
**Removal condition:** Replacement API implemented and behavioral tests pass.
**Review deadline:** `<version/date milestone>`

A crate-local pin overrides the workspace version. It MUST be recorded,
because it can hide a split dependency graph (`cargo tree -p <crate>`
disagrees with the rest of the workspace) and it adds behavioral-test
obligation for both versions' runtime paths.

## Step 6 - Run behavioral tests (compilation is not enough)

A clean compile is not a runtime pass; it proves only that the types line
up. Route startup, message-type, FFI, and formatting failures show up only
at runtime. Precondition: Step 3 compile passes (or the Step 5 pin is in
force). Commands and assertions: `references/runtime-tests.md`.

**Do** - mandatory runtime list, all nine, after ANY dep upgrade:

1. Version before config loading: `aphrodite --version` with no
   `aphrodite.toml`, then with an existing config; both exit 0.
2. Startup with an existing config; engine health; ONE compress/retrieve
   round trip.
3. Route registration incl. catch-all/fallback; no "Path segments must not
   start with *" panic.
4. WS ping/pong, text, binary through the proxy.
5. FFI init + thread interaction; GIL-release probe when pyo3 changed.
6. Marker/hash parity vs Python `hashlib.sha256`.
7. Plugin import/load in a fresh session; never a stale dylib/process,
   because a stale one loads the previous binary; drift guard `diff -q
plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`.
8. Release builds for every supported target, where possible.
9. Reinstall after rebuild: source the environment file; fresh build lands
   in `target/release`; `aphrodite setup` installs it.

**Verify**

```sh
aphrodite --version
echo "EXIT:$?"
aphrodite_stats 2> /dev/null | head -5
echo "EXIT:$?"
python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py
echo "EXIT:$?"
```

**Stop if** any runtime item fails. Do NOT proceed to Step 7 with a known
runtime break, because a decision recorded over a known break falsifies the
record.

**Recovery**: permitted - return to Step 4/5 with the runtime symptom as
new evidence (route behavior class); prohibited - marking the upgrade
complete on compile evidence alone, because the compile does not exercise
the runtime paths the list probes.

## Step 7 - Record the compatibility decision

Leave a durable, reviewable decision; the next upgrade starts from
evidence, not memory. Precondition: Step 6 passed (or the failure was
converted into a pin at Step 5).

**Do**

- Record for every changed dependency: exact before/after versions, failure
  class, action taken, and - for pins - the full `### Compatibility pin`
  entry (reason, affected surface, evidence, owner, removal condition,
  review deadline).
- Update `references/breakpoints.md` when a new breakpoint or a changed live
  state (e.g. a pin that landed) is discovered - it is the
  `verification.source_of_truth` for known breakpoints.
- Leave the workspace uncommitted (auto-committer sweeps; never commit/push
  from this skill).

**Verify**

```sh
grep -rn 'tokio-tungstenite\|pyo3\|sha2\|reqwest\|axum' references/breakpoints.md | wc -l
echo "EXIT:$?"
```

**Stop if** a decision lacks an owner or expiry, because the pin is then
ungoverned debt.

**Recovery**: permitted - write the missing ledger entry, then rerun the
Verify command; prohibited - deferring the ledger until the next upgrade,
because an unrecorded pin is indistinguishable from an accidental pin.

## CI parity gates

- Build.yml: 4-target matrix producing 12 assets - the release build must
  pass before any upgrade claim.
- Publish.yml: publish chain headroom-core → aphrodite → aphrodite-hermes
  under the `Aphrodite/v*` tag scheme; never publish out of chain order,
  because the chain is the dependency order and an out-of-order publish
  ships against an unpublished dependency.
- Test-gate parity: `cargo test -p aphrodite --lib setup::tests` exercises
  the setup path locally; run it after any upgrade touching config/setup
  code.

Core rule: if a claim cannot be tested, do not write an operational
instruction that relies on it.

## Test matrix (claim → test)

| Claim                                                                      | Evidence source                                        | Test                                                                       | Pass condition                                                          | Failure response                                                          |
| :------------------------------------------------------------------------- | :----------------------------------------------------- | :------------------------------------------------------------------------- | :---------------------------------------------------------------------- | :------------------------------------------------------------------------ |
| reqwest feature rename `rustls-tls` → `rustls` applies to crates/aphrodite | crates/aphrodite/Cargo.toml (0.13.5)                   | `cargo check -p aphrodite` after feature-string audit                      | No unknown-feature error; `cargo tree -p aphrodite -i reqwest` resolves | Revert feature rename; pin reqwest 0.12 locally                           |
| axum 0.8 wildcard syntax is `{*path}`                                      | route registration source                              | Start proxy; request an unmatched path                                     | No startup panic; catch-all/fallback behaves                            | Pin axum 0.7 workspace-wide; fix route strings                            |
| axum ConnectInfo fallback requires 0.7                                     | vendor/headroom workspace (`axum = "0.7"`)             | `cargo check --manifest-path vendor/headroom/Cargo.toml -p headroom-proxy` | Compiles with `any(catch_all)` + ConnectInfo                            | Keep the 0.7 pin; do not migrate fallback                                 |
| tokio-tungstenite Message types are Bytes/Utf8Bytes on 0.29                | headroom-proxy/Cargo.toml (0.24 pin, live)             | WS ping/pong/text/binary round trip through proxy                          | Payloads byte-identical; types compile                                  | Keep local 0.24 pin; migrate match arms only when bumped                  |
| pyo3 `allow_threads` removal is an ABI/FFI change                          | vendor/headroom workspace (`pyo3 = "0.29"`)            | FFI contract checker + GIL-release thread probe                            | `test_finalize_bindings.py` + `test_check_ffi_contract.py` green        | Pin pyo3 0.24; defer migration; rerun FFI gates                           |
| sha2 LowerHex removal breaks hex formatting                                | headroom-core (`0.11`) vs headroom-proxy/rtk (`0.10`)  | Marker/hash formatting test vs Python `hashlib.sha256`                     | Byte-identical hex at both versions                                     | Migrate to `iter().map(format!("{:02x}"))`; test both halves of the split |
| Crate-local pin overrides workspace                                        | headroom-proxy/Cargo.toml `tokio-tungstenite = "0.24"` | `cargo tree -p headroom-proxy` vs workspace tree                           | One resolved version per crate; split graph recorded in ledger          | Record split; add behavioral tests for both versions                      |
| Compilation alone proves nothing at runtime                                | Step 6 runtime suite                                   | Run the full mandatory runtime list after every upgrade                    | All 9 runtime items pass with recorded output                           | Halt upgrade; revert or defer the cluster                                 |
| Every pin has a lifecycle                                                  | Pin ledger (`### Compatibility pin` entries)           | Grep manifests for pinned versions; cross-check ledger                     | Every pin has owner + removal condition + review deadline               | Add the missing ledger entry before continuing                            |
| `--version` works before config load                                       | main() arg scan                                        | `aphrodite --version` with and without `aphrodite.toml`                    | Both print version, exit 0                                              | Fix early arg check; treat as CLI-startup breakpoint                      |
