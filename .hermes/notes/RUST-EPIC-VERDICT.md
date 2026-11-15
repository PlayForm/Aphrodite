# Rust rewrite epic - verification verdict

Measured at HEAD `ab8ab66a` on branch `Development`, against the working tree that carries the epic's
changes. Both crates: 2,072 `.rs` files, 51,121 lines under `crates/aphrodite/Source` and
`crates/aphrodite-hermes/Source`. Every number below was produced by the command printed beside it,
run in that tree; nothing is taken from a package card's handoff.

The tree had never been compiled before this card. The first `cargo ci-check` reported **150 errors**
in `crates/aphrodite` alone and 62 after the first repair pass; the ladder below is the state after
they were cleared.

## Ladder

| Command                                                                        | Exit | Note                                                                                                                                   |
| ------------------------------------------------------------------------------ | ---- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo ci-check`                                                               | 0    | `check --workspace --release`. First run: 150 errors, all mechanical (see the fixes section).                                          |
| `cargo test --workspace`                                                       | 0    | 623 passed, 0 failed, 1 ignored, across 10 targets.                                                                                    |
| `cargo ci-clippy`                                                              | 0    | With `clippy::match_bool` and `clippy::single_match` removed from both `Library.rs` roots. 13 lints and 42 shape sites cleared.        |
| `cargo ci-fmt`                                                                 | 0    | After `cargo fmt --all`: the first check reported 145 diffs in package files no card had formatted.                                    |
| `python3 Maintain/check_ffi_contract.py`                                       | 0    | `FFI contract check: 0 violation(s), 0 warning(s) -> PASS`.                                                                            |
| `python3 Maintain/tests/test_check_ffi_contract.py`                            | 0    | The checker's own suite.                                                                                                               |
| `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py`            | 0    | 23/23 OK. The ABI header did not change, so the generator skipped.                                                                     |
| `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` | 0    | Identical; the drift guard holds.                                                                                                      |
| `npx prettier --check '.hermes/**/*.md'`                                       | 1    | One file: `.hermes/skills/engineering/rust-module-atomization/SKILL.md`, committed and unmodified, and out of this card's write scope. |

Eight of nine are green. The ninth is a single pre-existing formatting failure in a directory this
card is forbidden to write; the four epic documents and every other `.hermes/**/*.md` are clean.

## Sweeps

| Claim                                                                     | Count                    | Residue                                                                    | Verdict |
| ------------------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------------------- | ------- |
| No dummy scrutinee, no artificial-error termination                       | 0                        | none                                                                       | PASS    |
| No boolean `match` with a no-op arm                                       | 0                        | none                                                                       | PASS    |
| Exported ABI symbols unchanged                                            | 24 / 11                  | none - the audit's HEAD count is reproduced exactly                        | PASS    |
| No lock held across a call that locks the same static                     | 70 hits, 64 real sites   | none in the self-deadlock class; see the note below                        | PASS    |
| The four epic documents exist, are prettier-clean, carry no machine paths | 4 OK                     | none                                                                       | PASS    |
| Structure: one item per file, facades declare only                        | 27 multi-item, 6 facades | 24 pre-existing; 3 new and each a `#[cfg]` const set or a struct+impl pair | PASS    |
| `if` sites reduced by at least 60% per package                            | 121 -> 56 (53.7%)        | E1, E4, E7, E9 and `Marker/` are above 40% of baseline; see the note below | PARTIAL |

### Sweep 1 and 2 - the forbidden shapes

`grep -rn 'match ()\|match true\b\|match false\b\|Err(())\|Ok::<(), ()>' crates/*/Source` returns
nothing, and the boolean-no-op probe returns 0. The audit's baseline was 20 dummy scrutinees and 23
artificial-error terminations; the last survivor was `Marker/Format/Entry.rs:50`, whose `match ()`
over four budget guards became a `match Budget` over four range arms - a scrutinee that carries the
number it decides on.

Sweep 2's grep is the coarse net. The verdict is `cargo ci-clippy` with the two crate-level allows
gone: it exits 0, and re-running it with the allows present would hide 42 sites that no longer exist.

### Sweep 3 - the ABI surface

`crates/aphrodite/Source/ABI` carries 24 `pub extern "C" fn` and `crates/aphrodite-hermes/Source/ABI`
carries 11, the same counts the work package records. `Maintain/check_ffi_contract.py` agrees, and
`diff -q` on the two `__init__.py` copies is empty.

### Sweep 4 - lock discipline

70 `.lock()` hits; 6 are doc-comment prose (`Proxy/State/Struct.rs`, `Retrieve/Handle/Lookup/Fn.rs`,
`Session/Lock/Fn.rs`), so 64 real acquisitions. Each was read and asked one question: does this guard
outlive a call that locks the same static?

- **`SESSION_PARENTS` (the D1 static).** Three acquisitions, none nested. `Debug/Chain/Fn.rs:20-25`
  holds the guard across `In::Fn::Fn` only, which takes the map by reference and locks nothing.
  `Debug/Session/Record/Resolve/Fn.rs:30-37` takes it inside the `or_else` closure and calls the pure
  walk there. `Debug/Session/Record/Fn.rs:29-34` holds `LAST_SESSION` in its own block, released
  before `Resolve::Fn::Fn` is called at `:43`.
- **`LAST_SESSION`.** `Debug/Session/Root/Fn.rs:25-32` clones out of the guard and drops it before
  `Chain::Fn::Fn`; `Debug/Session/Last/Fn.rs:29-39` scopes it and reads the fallback file after.
- **`SESSIONS`.** `Session/Broadcast/Fn.rs:21-28` collects the `Arc` clones inside the guard's scope
  and releases it before contending on any session mutex; `Session/Registry/Struct.rs:40-47` holds it
  for a map operation plus `config_template()`, which is a `OnceLock` and locks no mutex.
- **Per-session state mutexes.** `ABI/Handle/Lookup.rs:28` holds one across `f(&mut State)`, which is
  the contract; `GetHandle` runs at `:27`, before the lock. `Session/Test/Wedged.rs:20` holds one on
  purpose - that is the wedged-worker probe.
- **`FLAG_CACHE`, `RootSession`, `InlineCCR`, `ResponseCache`, `Scripts`.** Every site holds its guard
  across a filesystem read, a map operation or a `put`/`remove`, and none of those re-enters.
- **Test serialisers.** `Template/Guard`, `Home/Environment/Guard`, `Preview/State/Guard`,
  `Marker/Format/Const.rs:68`, `ConfigurationLoader/Test/*`, `Proxy/Test/Threshold`,
  `Retrieve/Test/*`, `Session/Test/*`, `Directive/Test/Guard`, `Health/Test/Port` - `Mutex<()>`
  guards a test takes and returns, with nothing inside that takes the same one.

The class the E9 card fixed is gone: no guard in either crate outlives a call into the same crate
that locks the same static.

### Sweep 6 - structure

The probe is the card's, run against the working tree rather than the index (the index still lists
files the rewrite deleted, so `git ls-files` reports 30 paths that no longer exist).

27 files carry more than one public item. 24 are pre-existing, and most are a `#[cfg]`-selected set
of definitions of the same constant (`Setup/Dylib/TargetTriple/Const.rs` five, `Copy/Names/Const.rs`
three) or a struct with its `impl` (`Session/Registry/Struct.rs`, `Health/Port/Fn.rs`). Three are new
since the epic's baseline - `Setup/Dylib/TargetTriple/Const.rs`, `Setup/Dylib/Copy/Names/Const.rs`,
`ABI/Handle/Registry/Struct.rs` - and each is the same legitimate pairing, not a collapsed leaf.

Six `mod.rs` facades carry something other than declarations. Three are test-only helper facades
(`Configuration/Test`, `Hook/Test`, `Preview/Test`) that hold a fixture builder and a guard accessor;
two are a module-inner attribute (`Proxy/mod.rs`, `Resolve/Recursive/mod.rs`); one is `use` lines in
`Resolve/Test/mod.rs`. None carries an item a consumer names.

### Sweep 7 - `if` sites

The probe is the work package's, per package, over the full tree under each prefix.

| Package        |  Before |  After |       Cut |
| -------------- | ------: | -----: | --------: |
| E1             |       5 |      5 |      0.0% |
| E2             |      20 |      3 |     85.0% |
| E3             |      10 |      0 |    100.0% |
| E4             |      12 |     12 |      0.0% |
| E5             |      31 |      3 |     90.3% |
| E6             |      20 |      6 |     70.0% |
| E7             |       3 |      2 |     33.3% |
| E9             |      16 |     16 |      0.0% |
| `Marker/` (C1) |       4 |      9 |       n/a |
| `(root)` (C2)  |       0 |      0 |       n/a |
| **Total**      | **121** | **56** | **53.7%** |

The number moves against the epic on four packages, and the reason is the same one in each: the
packages' own passes converted `if` into `match <bool> { true => work, false => () }`, which the
probe cannot see and which the style guide forbids. This card removed those shapes - 42 of them, the
last ones in the tree - by writing back the `if`/`if let` that clippy asks for once the crate-level
allows are gone. E1, E4, E9 and `Marker/` therefore gain the sites their forbidden shapes were
hiding. The two metrics pull in opposite directions by construction; the style guide settles it in
§2 ("counting eliminated `if` tokens measures the rewrite, not the code") and in the claim-to-test
table, where the boolean-match claim is settled by clippy and not by a count.

Read together the two numbers say what happened: 121 `if` sites and roughly 430 no-op arms became 56
`if` sites and no no-op arms.

The survivors were read one by one. Legitimate, with the reason:

- Match guards inside a genuine structural match: `Catalog/Size/Classify/Fn.rs:14-15` (an ordered
  threshold ladder over one scalar, each arm binding the count it classifies),
  `Setup/Dylib/Checksum.rs:36,79`, `Setup/Artifact/Stage/Fn.rs:37`, `Setup/Verify/Hermes.rs:24`,
  `Directive/Materialize/Fn.rs:53,61`, `Proxy/State/Build/Fn.rs:63`, `Marker/Format/Vars.rs:41`.
- Probe false positives - the word `if` inside a string literal, not code: `Flow/Prompt/Const.rs:18,23`,
  `Flow/Test/Prompt.rs:40,53`, `Schema/Directive/Fn.rs:21`, `Schema/Retrieve/Fn.rs:16`,
  `StructExtract/Test/Go.rs:29`, `Bin/SetupHelper.rs:29`.
- Guard clauses added by this card where a `match` was restating them: `Boot/Fn.rs:44` and the
  `if let` sites listed under the fixes. §2 makes `if` the correct form for a predicate.

E7 is the one package below the 60% floor on its own terms (3 -> 2). Its card recorded the reason:
60% of a three-site baseline is one site, and reaching it would mean re-introducing the banned shape.

## Defect register

| Id  | Verdict | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | CLOSED  | `Debug/Session/Record/Resolve/Fn.rs:30-37` takes `SESSION_PARENTS` inside the `or_else` closure and calls `Chain::In::Fn::Fn`, which is `std::iter::successors(...).take(17)` over a borrowed map (`Debug/Chain/In/Fn.rs:21-27`) - no lock, no global. `Debug/Session/Record/Fn.rs:23-46` holds no guard at all; it delegates to `Resolve` and logs after it returns. The only locking walk left is `Debug/Chain/Fn.rs:19-26`, for callers that hold no guard. A nested acquisition is unrepresentable. |
| D2  | CLOSED  | The dead arm is gone: `Boot/Listen/Watcher/Loop/Fn.rs:25` is `while let Some(()) = Rx.recv().await`, so a closed channel ends the controller. The audit's own verdict was REFUTED - nothing span at HEAD - so this is dead code removed, not a spin fixed.                                                                                                                                                                                                                                              |
| D3  | CLOSED  | One predicate decides streaming, from the parsed top-level field: `Proxy/Request/Semantics/Fn.rs:41` is `.and_then(\|Value\| Value.get("stream"))`. The byte-scan fast path no longer exists - `Proxy/Body/Stream/` is not in the tree. Two tests pin the case the register named: `Proxy/Test/Body/Fn.rs:78` (`TestDeliveryNestedStreamFlagDoesNotMakeRequestStreaming`) and `Proxy/Test/Cache/Fn.rs:88` (`TestSemanticsNestedStreamFlagDoesNotStream`).                                               |
| D4  | CLOSED  | The bound is owned by the buffer that mutates it: `PollWorker/Output/Struct.rs` defines `Accumulator { Buffer, Maximum }` whose `Append` decides what fits, so the buffer is never longer than `MaximumAccumulatedBytes` before or after an append. No caller tests a length.                                                                                                                                                                                                                           |
| D5  | CLOSED  | The record boundary terminates the subject scan: `Preview/Parse/GitLog/Flush/Fn.rs:13-18` emits only `HaveSubject` and resets to `Outside`, so a commit with an empty message contributes no endpoint instead of inheriting the next commit's subject. The state machine that feeds it is `Parse/GitLog/Transition/Fn.rs`.                                                                                                                                                                              |
| D6  | CLOSED  | A poll's exit code applies to the one task it names: `PollWorker/Apply/Fn.rs:16-20` resolves `Target(&Read(This), Event)?` and applies to that index only. The doc comment states the old defect by name.                                                                                                                                                                                                                                                                                               |
| D7  | CLOSED  | The accumulator is moved, not copied: `PollWorker/Output/Struct.rs:41` is `Take(&mut self) -> String { std::mem::take(&mut self.Buffer) }`. No clone of the buffer survives in `PollWorker/`.                                                                                                                                                                                                                                                                                                           |

## Package verdicts

| Package | What landed                                                                                                                                                                              | What is missing                                                                                                                                                                                |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E1      | ABI exports one per file with the 24 symbols intact, `Guard` rejections as `?` statements, `State` and `Logging` atomized. 0 forbidden shapes, clippy clean.                             | Nothing outstanding in its paths; its `if` count is 5 because three of this card's conversions landed there.                                                                                   |
| E2      | `Proxy/` rewritten as a parse-then-decide pipeline, `Request/Semantics` taking the streaming decision once (D3), the cache key and stats keys unchanged. 85% `if` cut.                   | `Proxy/mod.rs:35` keeps a new subtree-level `#![allow(clippy::obfuscated_if_else)]` over 16 `then_some(A).unwrap_or(B)` sites the style guide §8 rejects. OPEN.                                |
| E3      | `Hook/`, `ChainSplit/`, `Session/`, `Prefetch/` as one item per file; 0 forbidden shapes, 0 `if` sites, the hook envelope and marker text unchanged.                                     | Nothing outstanding.                                                                                                                                                                           |
| E4      | `PollWorker/` as decode -> apply -> complete with D4, D6 and D7 closed; `Flow/` windows as typed values.                                                                                 | Its `if` count is 12 after this card's conversions; four of those are string literals the probe cannot distinguish.                                                                            |
| E5      | `Preview/` as parse -> summary -> render, `Stage2/` reducers typed, `StructExtract/` declarations as a value, D5 closed. 90% `if` cut.                                                   | Two preview regressions shipped and were repaired here (see the fixes); nothing else outstanding.                                                                                              |
| E6      | `Boot/`, `ConfigurationLoader/`, `Configuration/`, `Setup/`, `Home/` atomized; D2's dead arm removed; the setup plan is an inspectable value. 70% `if` cut.                              | `Boot/Configuration/Load/Fn.rs` shipped a use-after-move this card repaired. The binary root keeps the two lints §1 revokes.                                                                   |
| E7      | `Directive/`, `Template/`, `Scripting/`, `Resolve/`, `Catalog/`, `Retrieve/` as modes and typed records; `limit: 0` understood in one place. 0 forbidden shapes.                         | 3 -> 2 `if` sites, below the 60% floor for the reason its card recorded. The builtin-directive table moved to `Directive/Embedded/Const.rs`, which broke a packaging test this card repointed. |
| E9      | The bridge crate rewritten as named leaf matchers, the session graph walked with `successors(...).take(17)`, D1 confirmed and the watcher split into debounce/apply. 0 forbidden shapes. | Its `if` count is 16 because eleven of this card's conversions landed there.                                                                                                                   |

## Mechanical fixes applied by this card

The first build reported 150 errors in `crates/aphrodite` and none of them was a missing module
declaration: the module tree was complete, and every error was a consumer naming a leaf that had
gained a kind segment, or a visibility that stopped at the wrong parent.

- **Stale paths, 118 sites.** The dominant shape: `X::Enum::Variant` where the leaf is
  `X/Enum.rs` declaring `enum Enum`, so the path is `X::Enum::Enum::Variant`; the same for
  `Struct`, `Const` and `Fn`. Files: `Setup/{Plan,Execute,Register,Artifact,Dylib/Copy}/**`,
  `Catalog/{Build,Size,Mode}/**`, `Retrieve/{Handle,Page,Live}/**`, `Resolve/{Recursive,Query,Filter}`,
  `Stage2/{Reduce,Compress}/**`, `StructExtract/Extract.rs`, `Preview/Parse/GitLog/**`,
  `Boot/Configuration/**`, `Boot/{Request,Run,Fn}.rs`, `Directive/{Build,Builtin,Cap,Verb}/**`,
  `Setup/Execute/Binary`, `Setup/Write/BinaryName`. 22 of them were a `super` that had gained a
  level when a leaf became a directory (`Directive/Verb/{Add,Load,Swap}/Fn.rs`,
  `Catalog/Size/Classify/Fn.rs`, `Resolve/Query/Match/Fn.rs`, `Retrieve/Page/{Slice,Render/*}/Fn.rs`,
  `Retrieve/Source/Parse/Fn.rs`, `Resolve/Recursive/Prior/Classify/Fn.rs`,
  `Preview/Parse/GitLog/{Flush,Transition}/Fn.rs`).
- **Half-applied conversions, 21 sites.** `pub(super)` widened to `pub(crate)` where the consumer is a
  sibling (`Setup/Dylib/Copy/{Locate/Fn,Locate/Target/Fn,Install/Fn,Unavailable/Fn,Names/Const,Source/Enum,Requirement/Enum}`,
  `Setup/Dylib/{Download/Fn,TargetTriple/Const}`); `Directive/Embedded/Const.rs` `include_str!`
  repointed to `../../BuiltinDirective/`; `Setup/Execute/Fn.rs` repointed to `Dylib::Copy::Fn::Fn`;
  `Boot/Listen/Watcher/Source/Fn.rs` importing the item rather than the module.
- **`Boot/Configuration/Load/Fn.rs` - a use-after-move, and the one design break.** The standalone arm
  built `Source::Standalone(CLI)` and then read `CLI.LogCompact` in the same struct literal. The fix
  resolves the flag before the source takes ownership. This is the only error the full build revealed
  that was not a naming mismatch, and it did not need a redesign.
- **`Preview/Render/Cap/Fn.rs` - an ambiguous lifetime.** `Cow<'_, str>` from two elided input
  lifetimes; bound to `'a` on the preview and its return.
- **`Preview/Render/Listing/Fn.rs` - an inverted condition (behaviour).** `Summary.IsEmpty().then(...)`
  renders the body for an _empty_ listing and the bare line count for a full one. Six preview tests
  failed on it (`TestDetectAndPreviewLsLong` in isolation). Now `(!Summary.IsEmpty()).then(...)`, which
  restores the documented contract: a listing that recognised nothing renders `[ls:NL]`.
- **`Preview/Builder/{JSON,CSV,YAML,Diff,Table}.rs` - `+0 more` (behaviour).** Five sites formatted the
  remainder unconditionally, so a preview with exactly as many items as it shows read `keys: a, b +0
more`. `Render/Git/Fn.rs` already carried the `.filter(|More| *More > 0)` these five lacked; the same
  filter is now on all of them. Six preview tests failed on this.
- **`crates/aphrodite/tests/packaging.rs` - a stale path.** It read the `include_str!` table from
  `Source/Directive/Builtin.rs`, which the epic moved to `Source/Directive/Embedded/Const.rs`, and its
  normaliser stripped one `../` prefix where the new location needs two.
- **`PollWorker/Test/Update/Live.rs` - a cross-test race.** `TestPollInstantMarkerHashAvailableBeforeContent`
  renders a marker through the process-global template registry without the guard the `Marker` tests
  hold, so it failed in the full run and passed alone. It now takes `MarkerTemplatesTestGuard`.
- **`PollWorker/Test/Update/Live.rs`, `Marker/Format/Entry.rs` and 39 other sites - the last forbidden
  shapes.** With the crate-level allows gone, clippy reported 42 `match bool` / single-pattern sites
  across 31 files; each became the `if`/`if let`/`then` form the lint asks for, with no type or
  signature change. `Marker/` is the C1-unassigned module: no card owned those eight sites, and they
  are the only ones this card fixed outside a package's declared prefixes.
- **`crates/aphrodite/Source/Library.rs` and `crates/aphrodite-hermes/Source/Library.rs` - the two
  crate-level allows removed.** This is the write the work package assigns to this card (§5 C2). With
  them gone, `cargo ci-clippy` exits 0 on both crates.
- **Formatting.** `cargo fmt --all` over 145 diffs no card had formatted.
- **Three `.hermes/notes` documents** formatted so sweep 5 holds: `RUST-STYLE-FUNCTIONAL.md`,
  `APHRODITE-MESSAGE-CATALOGUE.md`, `APHRODITE-PIPELINE-PARAMETERS.md`.
- **One message change.** `Setup/Execute/Registration/Fn.rs` now names the exit code in its warning
  (`hermes plugins enable aphrodite (exit N): ...`) because `Outcome::ExitedNonZero`'s `Code` field was
  never read and clippy failed the gate on it. Every other error and warning string in the tree is
  unchanged.

## Residue

Findings that stand, each with the reason it is not a fix.

- **`npx prettier --check '.hermes/**/*.md'` fails on one file.** `.hermes/skills/engineering/rust-module-atomization/SKILL.md`
  is committed, unmodified by the epic, and inside a directory this card must not write. The repair is
  `npx prettier --write` on that one path.
- **`Proxy/mod.rs:35` keeps `#![allow(clippy::obfuscated_if_else)]`, added by E2.** It suppresses 16
  sites that spell a two-way choice `Condition.then_some(A).unwrap_or(B)`, a form §8 rejects outright.
  Removing the allow reddens `cargo ci-clippy` with those 16; converting them is E2's work, and the
  sites are `Proxy/Analysis/Classify/Envelope/Fn.rs:21`, `CCR/Create/Decode/Fn.rs:19`,
  `CCR/Notification/Deliver/Fn.rs:29`, `Cache/Freshness/Classify/Fn.rs:15`,
  `Compress/Chat/Plan/Fn.rs:139`, `Handle/Proxy/Respond/Buffered/Fn.rs:124`,
  `Handle/Proxy/Route/Fn.rs:50`, `Metadata/Build/Fn.rs:8`, `Metadata/JSON/Fn.rs:52`,
  `Relay/Handle/Route/Fn.rs:98`, `Request/Route/Classify/Fn.rs:12`, `Request/Semantics/Fn.rs:39`,
  `State/Method/Fill/Fn.rs:19`, `Storage/Store/Fn.rs:40`, `Storage/Target/Classify/Fn.rs:10`,
  `Upstream/Media/Fn.rs:13`.
- **`crates/aphrodite/Source/Main.rs` keeps both revoked lints.** The style guide §1 and the work
  package §5 C2 name the two `Library.rs` roots, not the binary's own root. Removing them here surfaces
  37 further sites under `Boot/` and `Guard/`, which is the same E6 work again in a path no card
  declared; the file carries a comment saying so.
- **`Resolve/Recursive/mod.rs:25` keeps `#![allow(clippy::module_inception)]`.** Pre-existing at the
  epic's baseline, and the only way to name a leaf `Recursive/Recursive.rs` under the house style.
- **Two allows this card added, each with its reason in place.** `StructExtract/Declaration/Enum.rs`
  carries `#[allow(dead_code)]` on `Malformed::Reason` (the variant is constructed by eleven
  recognisers and rendered by none, so the reason is diagnostic state nothing reads yet);
  `Boot/Configuration/Source/Enum.rs` carries `#[allow(clippy::large_enum_variant)]` (both payloads are
  configuration structs of comparable size, so boxing one moves the imbalance rather than removing it).
- **27 files carry more than one public item.** 24 predate the epic and most are a `#[cfg]`-selected
  set of definitions of one constant or a struct with its `impl`; the three the epic introduced are the
  same shape. None is a leaf that collapsed two items into one file.
- **The `if` count is above the 60% floor on five prefixes.** The trade-off is stated under sweep 7:
  finishing the conversion the packages left half-done writes `if` back where a forbidden `match bool`
  stood, and the style guide makes the forbidden-shape claim the one clippy settles.
- **`Setup/Execute/Registration`'s warning text changed.** It is the one string this card altered, to
  satisfy the dead-code lint on the outcome's exit code.
- **The epic's packages never ran their own gates.** Every package's acceptance ladder ended in
  `cargo clippy -p ... -- -D warnings` and `cargo fmt --all -- --check`, and the tree reached this card
  with 13 clippy failures, 42 forbidden shapes hidden behind crate-level allows, and 145 formatting
  diffs. That is why this card carries a build mandate at all, and it is the one finding that is about
  the programme rather than the code.
