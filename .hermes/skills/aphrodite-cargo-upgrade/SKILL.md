---
name: aphrodite-cargo-upgrade
description: "Use when upgrading cargo deps in PlayForm/Aphrodite (crates/aphrodite + vendor/headroom owned fork). Decision tree: inventory, one compatibility cluster at a time, minimal-target compile, failure classification, migrate/pin/revert/defer, runtime behavioral tests, pin lifecycle."
version: 1.2.0
platforms: [macos]
tags: [aphrodite, cargo, upgrade, rust, breakpoints, pinning]
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
supersedes:
    - aphrodite-upgrade-breakpoints
verification:
    source_of_truth:
        - workspace and crate Cargo.toml manifests, Cargo.lock
        - references/breakpoints.md (known breakpoints, manifest-verified)
        - runtime probes: aphrodite --version, route/WS/FFI/hash round trips
mutation_level: local
---

# Aphrodite Cargo Upgrade (Decision Tree)

Canonical procedure for dependency upgrades in PlayForm/Aphrodite: the parent
workspace (`crates/aphrodite`, `crates/aphrodite-hermes`) and the
`vendor/headroom` OWNED fork (a submodule that may be modified freely,
edition-2024 workspace). Entry point used historically:
`~/Developer/Maintain/Fn/Update/Cargo.sh` (environment-specific; its
`ExpandVersions` helper hits the toml_edit breakpoint in
`references/breakpoints.md`).

Anti-pattern this tree prevents: "pin everything until it compiles", which
leaves a split dependency graph and untested runtime breakage.

## Preflight (delegated - do not duplicate)

Before Step 1, run the **aphrodite-orientation** gate (repository root,
permitted branch `Development`, clean/known worktree, submodule status,
auto-committer HEAD snapshot). Apply the **aphrodite-boundaries** stop and
recovery rules at every step: never mutate before a precondition is verified,
never continue after a failed verification, never use `git checkout`/`reset`
to repair content. `mutation_level: local` - no commit/tag/push here; the
auto-committer sweeps.

## Step 1 - Inventory the upgrade surface

**Purpose**: Capture the exact pre-upgrade state so every later decision has a
baseline.

**Preconditions**

- Orientation gate passed (aphrodite-orientation).
- No unrelated working-tree changes in manifests or Cargo.lock.

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

**Expected**

- Lockfile diff, package graph, toolchain version, and the full manifest list
  (parent crates + headroom workspace + headroom crates + vendor/rtk) are
  recorded.
- No uncommitted manifest edits from a previous session.

**Stop if**

- The lockfile or a manifest differs from what the diff shows (auto-committer
  race) - re-capture, do not proceed on stale evidence.

**Recovery**

- Permitted: re-run the inventory read-only commands; repair intended content
  by editing the file directly.
- Prohibited: `git checkout`/`git restore`/`git reset` to "clean" the diff.

**Produces**: baseline evidence record for the upgrade session.

## Step 2 - Upgrade one compatibility cluster at a time

**Purpose**: Bound blast radius so a failure is attributable to one group of
co-changing dependencies.

**Preconditions**

- Step 1 baseline recorded.

**Do**

```sh
cargo upgrade --dry-run <cluster>   # e.g. axum, or tokio-tungstenite alone
# then, for the accepted cluster only:
cargo update -p <cluster-member> --dry-run
```

**Expected**

- The proposed change touches ONE compatibility cluster: networking
  (reqwest/tokio-tungstenite), web routing (axum), Python FFI (pyo3/pyo3-log),
  crypto/hash (sha2), or an unrelated single crate.

**Stop if**

- The upgrade would move unrelated networking + FFI + crypto + web deps
  together - that requires an explicit compatibility matrix (each crate
  compiled AND runtime-tested at the proposed versions) before it is allowed.

**Recovery**

- Permitted: split the request into per-cluster `cargo upgrade` runs.
- Prohibited: pinning the whole graph "until it compiles" as a substitute for
  cluster isolation.

**Produces**: one accepted cluster change (or a refused one, ending the run).

## Step 3 - Compile minimal targets independently

**Purpose**: Prove each layer of the graph compiles before any runtime claim.

**Preconditions**

- Step 2 accepted exactly one cluster.

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

**Expected**

- Workspace package, affected crate, FFI boundary, and vendored submodule
  each compile independently; the failing crate is isolated.

**Stop if**

- A failure's cause cannot be assigned to the cluster from Step 2.

**Recovery**

- Permitted: proceed to Step 4 classification with the isolated failure.
- Prohibited: "fixing" a different cluster's manifest to make this one pass.

**Produces**: per-target compile evidence and the failure surface for Step 4.

## Step 4 - Classify the failure

**Purpose**: Map the symptom to one failure class, because each class has one
allowed action.

**Preconditions**

- Step 3 produced a compile error or a known runtime suspicion.

**Do** - classify against this table (examples in `references/breakpoints.md`):

| Class                  | Recognition                                                                               | Default action                |
| :--------------------- | :---------------------------------------------------------------------------------------- | :---------------------------- |
| API rename             | symbol/type moved or renamed (e.g. `Message` variants, `DocumentMut`)                     | migrate source                |
| Trait-bound change     | `trait bound ... not satisfied` (e.g. `LowerHex`, axum `Handler`, tracing `DisplayValue`) | migrate source or pin locally |
| Feature rename         | `dep does not have that feature` (e.g. reqwest `rustls-tls` → `rustls`)                   | migrate feature string        |
| Semver incompatibility | behavior/API removed across major bump without replacement                                | pin locally or revert         |
| Runtime route behavior | starts but routes/fallback/WS misbehave                                                   | pin + behavioral tests        |
| ABI/FFI change         | pyo3/FFI surface change (e.g. `allow_threads` removed)                                    | pin; FFI contract gates       |
| Security advisory      | `cargo audit`/advisory for the target version                                             | pin to patched minor or defer |

**Expected**

- Exactly one class is chosen; the evidence (error text or failing probe) is
  recorded next to the dependency.

**Stop if**

- Two classes both plausibly match - resolve by compiling the crate at the old
  version to isolate the change.

**Recovery**

- Permitted: `cargo tree -p <dep>` and `cargo update --dry-run` to inspect.
- Prohibited: guessing from stale skill prose; re-derive from the checked-out
  manifest (fact class: source-derived).

**Produces**: one classified failure with attached evidence.

## Step 5 - Choose the action

**Purpose**: Select the single allowed response for the class, with a pin
lifecycle when pinning.

**Preconditions**

- Step 4 classification recorded.

**Do** - apply the default action from Step 4; when the action is a pin,
create its ledger entry immediately (template below). Action precedence:
migrate source > pin locally > pin workspace-wide > revert > defer.

**Verify**

```sh
grep -n '^<dep> *=' crates/*/Cargo.toml vendor/headroom/Cargo.toml vendor/headroom/crates/*/Cargo.toml
echo "EXIT:$?"
```

**Expected**

- The chosen action is applied in exactly the manifests it names; a pin has a
  `### Compatibility pin` entry (below) with owner and removal condition.

**Stop if**

- A pin would be added without a ledger entry (unexplained pins become
  permanent debt and hide split graphs).

**Recovery**

- Permitted: edit the manifest and ledger directly; rerun Step 3.
- Prohibited: pinning "temporarily" without recording expiry/revisit.

**Produces**: migrated source or a recorded pin/revert/defer decision.

### Compatibility pin

**Dependency:** `pyo3`
**Pinned version:** `<exact version>`
**Reason:** Required API was removed upstream; migration not yet implemented.
**Affected surface:** Python callback/thread boundary.
**Evidence:** Failing migration compile/test case.
**Owner:** `<team or area>`
**Removal condition:** Replacement API implemented and behavioral tests pass.
**Review deadline:** `<version/date milestone>`

A crate-local pin overrides the workspace version and MUST be recorded - it
can hide a split dependency graph (`cargo tree -p <crate>` disagrees with the
rest of the workspace) and increases the behavioral-test obligation to both
versions' runtime paths.

## Step 6 - Run behavioral tests (compilation is not enough)

**Purpose**: Prove runtime behavior, not just type-correctness. A clean compile
misses route startup, message-type, FFI, and formatting failures.

**Preconditions**

- Step 3 compile passes (or the pin from Step 5 is in force).

**Do** - mandatory runtime list, all of it, after ANY dep upgrade:

- `--version` path BEFORE config loading: run `aphrodite --version` with no
  `aphrodite.toml` present, then with an existing config - both must print the
  version and exit 0.
- Startup with an existing configuration file (real `aphrodite.toml`); then
  engine health (`aphrodite_stats` equivalent) and ONE compress/retrieve round
  trip through the proxy.
- HTTP route registration, including catch-all/fallback: start the proxy, hit
  a real route and an unmatched path; catch-all must behave, no
  "Path segments must not start with *" panic.
- WebSocket ping/pong, text, and binary messages through the proxy
  (tokio-tungstenite message types).
- Python/FFI init and thread interaction: `test_finalize_bindings.py`,
  `Maintain/tests/test_check_ffi_contract.py`, plus a GIL-release/thread probe
  when pyo3 changed.
- Marker/hash formatting and checksums: byte-identical hex vs Python
  `hashlib.sha256` (smart_crusher `_hash_field_name` parity).
- Plugin import/load: fresh Hermes session with the plugin (never a stale
  dylib/process), plus `diff -q plugins/aphrodite/__init__.py
crates/aphrodite/templates/__init__.py` (drift guard).
- Release builds for every supported target, where possible
  (`cargo build --release -p aphrodite`, plus per-platform builds when
  available).

**Verify**

```sh
aphrodite --version
echo "EXIT:$?"
aphrodite_stats 2> /dev/null | head -5
echo "EXIT:$?"
python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py
echo "EXIT:$?"
```

**Expected**

- Every item above passes with recorded output; nothing is inferred from a
  successful compile.

**Stop if**

- Any runtime item fails: do NOT proceed to Step 7 with a known runtime break.

**Recovery**

- Permitted: return to Step 4/5 with the runtime symptom as new evidence
  (route behavior class).
- Prohibited: marking the upgrade complete on compile evidence alone.

**Produces**: runtime evidence set attached to the upgrade.

## Step 7 - Record the compatibility decision

**Purpose**: Leave a durable, reviewable decision so the next upgrade starts
from evidence, not memory.

**Preconditions**

- Step 6 runtime suite passed (or a failure was deliberately converted into a
  pin at Step 5).

**Do**

- Record for every changed dependency: exact before/after versions, the
  failure class, the action taken, and - for pins - the full `### Compatibility
pin` entry (reason, affected surface, evidence, owner, removal condition,
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

**Expected**

- Every pinned dependency has a ledger entry with removal condition and review
  deadline; every new/verified breakpoint is in `references/breakpoints.md`.

**Stop if**

- A decision lacks an owner or expiry - the pin is then ungoverned debt.

**Recovery**

- Permitted: write the missing ledger entry, then rerun the Verify command.
- Prohibited: deferring the ledger "until the next upgrade".

**Produces**: the compatibility-decision record for this upgrade.

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

Core rule (rewrite.md): if a claim cannot be tested, do not write an
operational instruction that relies on it.
