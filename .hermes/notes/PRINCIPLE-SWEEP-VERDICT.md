# Principle sweep V - verification verdict

Nine sweep cards rewrote the tree without compiling it. This card is the first build of that programme, the arbitration of what the cards claim, and the verdict on each of them.

Nothing here re-runs a card's redesign. Where a card was half-converted, the conversion was completed minimally and the fact is recorded below.

## The ladder

Every gate was run from the repository root, with the command the CI workflow names, and its exit code read directly - never through a pipe.

- **`cargo ci-check` → 0.** `check --workspace --release`. Green after the fixes below.
- **`cargo test --workspace` → 0.** 558 library tests plus 17, 5, 2, 4, 2, 83, 0 and 3 across the integration and binary targets; 1 ignored, 0 failed.
- **`cargo ci-clippy` → 0.** `clippy -p aphrodite -p aphrodite-hermes -- -D warnings`. Green after the warning fixes recorded under _The first build_.
- **`cargo ci-fmt` → 0.** `fmt --all -- --check`. 135 files needed formatting after the sweep's hand-edits; `cargo fmt --all` was run once, then the check was re-run green.
- **`python3 Maintain/check_ffi_contract.py` → 0.** `0 violation(s), 0 warning(s) -> PASS`, 10 exports checked, 7 required, 6 called with JSON.
- **`python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py` → 0.** `Ran 23 tests ... OK`.
- **`diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` → 0.** Identical; the loader template did not drift.

### The Linux leg was not run, and why

`x86_64-unknown-linux-gnu` is installed as a rustup target, but the workspace pulls `aws-lc-sys`, whose build script shells out to `x86_64-linux-gnu-gcc`. That cross C compiler is not on this machine, so `cargo clippy --target x86_64-unknown-linux-gnu` fails inside the build script before any Rust is checked. A green local clippy is therefore not evidence for CI's Linux job.

Instead every `#[cfg]`-gated site in both crates was read - 31 of them, all in `crates/aphrodite/Source/Setup/` and `crates/aphrodite/Source/Boot/`. Each is a statement- or item-level selection whose bindings are used by the arm that is compiled:

- `Setup/Dylib/Copy/Install/Fn.rs` and `Setup/Execute/Binary/Fn.rs` gate the copy treatment; `Name`, `Src`, `Dest` and `Context` are used by both arms.
- `Setup/Artifact/Stage/Fn.rs` and `Setup/Dylib/Checksum.rs` gate the download and hash commands; `Name` and `Dest` are used by both.
- `Setup/Write/Permissions.rs` uses `(path, Mode)` on unix and re-binds them with `let _ = (path, Mode);` on the non-unix path, so neither is unused there.
- `Setup/Dylib/Download/Fn.rs` defines `Extension` three times under mutually exclusive cfgs.
- `Setup/Write/BinaryName.rs`, `Setup/Dylib/Copy/Names/Const.rs`, `Setup/Dylib/TargetTriple/Const.rs` and `Setup/MacOS.rs` are item-level selections.
- `Boot/Listen/Shutdown.rs` defines `Terminate` under `cfg(unix)` and `cfg(not(unix))`.

No `cfg`-gated binding is left unused on the Linux arm.

One Linux-shaped defect did surface, and it was not a `cfg` warning: `ConfigurationLoader/Configuration/Apply/Global/Rendering/Struct.rs` imported `crate::Configuration::Proxy::Multi::Struct::Struct` unconditionally, while `crate::Configuration` is behind the `proxy` feature. The workspace build unifies features and never noticed; `cargo test -p aphrodite-hermes` alone failed with `E0433`. The engine-side conversion is now `#[cfg(feature = "proxy")]`, which is where it belongs - the `Multi` document only exists in the proxy build.

## The first build

137 errors: 1 parse error, 108 errors in the library and binary targets, 28 in the test targets. They arrive in batches, because cargo stops at the first parse error and then reports per-crate, so the count is what four `cargo check` rounds and two `cargo test --no-run` rounds produced.

By kind:

- **Missing module declarations: 0.** Every new leaf is declared by its parent `mod.rs`. The one unresolved import (`Marker/Format/Registration/With/Fn.rs`) was a path that resolved one level too shallow - `super::Const` instead of `super::super::Const` - not a missing declaration.
- **Stale leaf paths: 119.** One shape, repeated: a consumer named the leaf module where the type inside it was meant. `Outcome::Enum` instead of `Outcome::Enum::Enum`, `Report::Struct` instead of `Report::Struct::Struct`, `Target::Enum`, `Mode::Enum`, `Payload::Enum`, `State::Enum`, `Identity::Enum`, `Event::Enum`, `Ready::Enum`, `Start::Enum`, `Handle::Struct`, `Ownership::Enum`, `FlagState::Enum`. 39 `E0425`, 44 `E0531`, 23 `E0573`, 6 `E0422`, 4 `E0574`, 3 `E0423`. This is the whole of B9's new type surface: not one of the types that card introduced was referenced correctly anywhere, which means none of them had ever been type-checked in a consumer.
- **Half-applied conversions: 18.** A stray `});` left behind in `Debug/Log/At/Fn.rs` where a closure had been removed but its terminator kept - this was the single parse error, and it hid every other error until it was fixed. `StructExtract/Format/{Signature,Render}/Fn` and their test still imported `Format::Cap::Fn` after `Cap` became a directory leaf. `Setup/Execute/Plugin/Directory/Fn.rs` matched `Plan.Ownership` by value behind a `&`. `ChainSplit/Test.rs` still compared owned `Part` values against `(usize, String)` tuples. `PollWorker/Test/Update/{Poll,Attribution}.rs` still passed a `String` where `CommandID` is required and still matched `BgStatus::Done { ExitCode, .. }` after the exit code moved to `Failed`. `StructExtract/Test/Format.rs` and `Tool/Registry/Test/Fn.rs` imported their enums one level short. `Guard/ToJSON/Fn.rs` used `c_char` with no import. `Guard/Loopback/Fn.rs` and its test passed `&*Hostname` and `&Hostname` where `Hostname` is a `Cow<str>`. `Tool/Unwrap/Search/Rows/Text/Fn.rs` returned the unwrapped `Path` where the fold accumulator holds `Option<&str>`. `Debug/Flag/State/Enum.rs` ended with a `const _` that referenced `Mutex` and `FLAG_CACHE` from a module that does not import them. `E0308` 8, `E0369` 3, `E0277` 3, `E0507` 1, `E0026` 1, `E0432` 1, plus the parse error.
- **Genuine design breaks - two cards disagreeing about a type: 0.** No two cards defined the same type incompatibly. The cross-card damage was of a different shape and is named under _Where two cards met_.

The build went green after 137 fixes with no redesign. Four defects found later, by the gates rather than by the compiler, are also part of this card's work:

- `Proxy::Cache::Identity::Struct` was `pub(crate)` while a `pub` field of `Proxy::State` holds an `LruCache` of it - a `private_interfaces` error under `-D warnings`. The type is now `pub`.
- `Prefetch::Prepared::SourceSize` was carried from the read through the prepare into the apply, and read by nothing: `Prefetch/Apply/Fn.rs` destructured it as `SourceSize: _`. The stat size is now carried only where it is used - `ReadOutcome::SkippedSize`, which is the refusal the caller reports.
- `Applied::Progressed(Subject)` was never read: the interpret step matched `Progressed(_)` and returned. The task the poll moved is now named at debug level, which is the one thing that payload was for.
- `Boot::Listen::Watcher::Outcome::Inactive { Cause }` carried a reason that was already logged where the watch was created and was discarded by the only consumer. It is now a unit variant; the warning keeps the reason.

## The six behaviour proofs

**1. Preview equality.** `"[abc]"` at limit 5 stays `"[abc]"`; `"abc"` at 3 stays; a multibyte preview at its scalar count stays; empty stays.

```sh
cargo test -p aphrodite --lib -- Preview::Test::Cap
```

6 passed, 0 failed. `TestPreviewCapFitsUnchangedAtBoundary` is the equality case, `TestPreviewCapIdentityAtAndBelowLimit` is the property over limits 1..=16 for plain and bracketed previews, `TestPreviewCapMultibyteTruncatesOnCharBoundary` is the scalar-count case, `TestPreviewCapUnsetIsUnlimited` is the absent cap. The equality defect the card names is fixed: `"[abc]"` at 5 was `"[...]"` and is now itself.

**2. All-skipped reload.** With every session lock unavailable, the watcher must not stop.

```sh
cargo test -p aphrodite-hermes --lib -- Session::Test
```

14 passed, 0 failed. The three rows of the distinction are separate cases: `probe_a_sweep_that_could_not_look_keeps_watching` (every lock wedged), `probe_an_empty_registry_keeps_watching` (no session registered yet), `probe_a_parse_failure_keeps_watching` (the file did not parse), against `probe_every_reached_session_turning_reload_off_stops` (the file really said stop). `probe_wedged_session_lock_degrades_fail_open` proves the sweep reaches the policy with a skipped count rather than blocking.

**3. Mixed publication.** A reader must not observe family B with templates A.

No test settles this one, and that is a finding: neither B4 nor B5 shipped the "barrier-controlled mixed-state regression" its batch table requires. The proof is structural, and it is exact: `Template::Render::Fn` takes one read guard and reads `St.Templates` and `St.Family` from that one guard, and `ConfigurationLoader/Configuration/Apply/Global/Publish/Fn::Fn` writes both fields under one write guard on the same `RwLock`. A reader therefore observes the previous pair or this one. What the sweeps did add is the single publication point - before it, `Template::Set` and `Template::Family::Set` each took the lock separately, which is precisely how the mixed state was reachable.

**4. Cap ownership.** Unlimited, fitting, oversized and multibyte produce identical output before and after the `Cow` change.

```sh
cargo test -p aphrodite --lib -- StructExtract::Test::Format
```

3 passed, 0 failed. `TestCapsFitsUnchangedAtBoundary` covers a parameter list at exactly 35 bytes and a signature at exactly 60 bytes (both whole, where the previous `checked_sub` lost their last three characters to `...`), and the 36- and 34-byte neighbours. `TestFloorBoundaryNeverPanicsOnMultibyte` covers the byte-offset cut. The `Cow` change is `Format::Cap::Fn` taking and returning `String`: both call sites already owned their string, so the fitting case - the common one - no longer copies, and the rendered bytes are unchanged.

**5. Poll accumulation.** One oversized poll cannot exceed the accumulator cap.

```sh
cargo test -p aphrodite --lib -- PollWorker::Test::Output::Boundary
```

5 passed, 0 failed. `TestAFullAccumulatorTakesNothing` is the oversized poll, `TestTheSeparatorIsChargedToTheBound` is the case where the separator would otherwise push the buffer one byte over, `TestThePrefixIsCutOnACharacterBoundary` is the multibyte cut, `TestTheFirstPollIsNotChargedASeparator` is the first-poll boundary, `TestTakeLeavesAnEmptyAccumulatorAtTheSameBound` is the moved-out buffer.

**6. One exit code, one task.** A poll carrying an exit code finishes exactly the task it belongs to.

```sh
cargo test -p aphrodite --lib -- PollWorker::Test::Apply::Transitions
```

5 passed, 0 failed. `TestOnePollFinishesOneTaskAndLeavesTheOthersRunning` is the two-task case: the polled task completes with its own exit code, the other keeps running and is not polled. `TestAPollForAnUntrackedCommandAppliesToNothing` is the mis-attribution case. `TestACompletionPollOwnsTheAccumulatedOutput` and `TestAProgressPollOwnsNoOutput` are the two variants' ownership. `TestAPollAfterCompletionAppliesToNothing` was repaired here: it asserted that a finished task is no longer a target without ever running the step that makes a task finished, so it now drives `Registry::Complete` between the two polls - the same two calls `UpdateFromPoll` makes.

## The three sweeps

**No dummy scrutinee, no artificial-error termination.**

```sh
grep -rn 'match ()\|Err(())' crates/*/Source --include=*.rs | wc -l
```

0. Both shapes are gone from the tree.

**No predicate that ignores its subject.**

```sh
grep -rn 'filter(|_| ' crates/*/Source --include=*.rs
```

5 hits, and 3 of them are not code:

- `crates/aphrodite-hermes/Source/Tool/Unwrap/Matcher/Sole/mod.rs:11` - a doc comment quoting the removed shape.
- `crates/aphrodite/Source/Catalog/Test.rs:146` - a test fixture; the research document names this exact line as the shape that replaced it, and a fixture's special case belongs in a `match` on the index.
- `crates/aphrodite/Source/Preview/Builder/Test/Failure/Fn.rs:20` - a test file, and there the predicate is `Trimmed.ends_with("FAILED")`, which ignores the `strip_prefix` result on purpose: it asks whether the shape matched, not whether the remainder is anything.

Two production sites survive, and both are the forbidden shape:

- `crates/aphrodite/Source/Proxy/Handle/Proxy/Respond/Buffered/Fn.rs:167` - `.filter(|_| Head.Status.is_success() && Payload.len() <= ResponseCacheMaximumBodyBytes)`. The predicate ignores the value it filters; it is an `if` guarding the `Some`.
- `crates/aphrodite/Source/Preview/Type/Fn.rs:39` - `.filter(|_| !Inp.raw.lines().any(Failure::Fn))`, same shape, in B4's tree.

**One item per file.**

```sh
git ls-files 'crates/*/Source/**/*.rs' | grep -vE '/(mod|Library|Main)\.rs$' | while read -r f; do
	n=$(grep -cE '^pub(\(crate\))? (fn|struct|enum|trait|const|static|type) ' "$f")
	[ "$n" -gt 1 ] && echo "$n $f"
done | sort -rn | head -20
```

The head of the list is dominated by constant files, which are the tree's own exception: `Proxy/Threshold/Const.rs` 9, `State/Const.rs` 7, `Setup/Dylib/TargetTriple/Const.rs` 5, `Setup/Dylib/Copy/Names/Const.rs` 3, `Hook/Live/Const.rs` 3. Of the remaining entries only two are not constant or test files:

- `crates/aphrodite/Source/Hook/ToolResult/Entry/Fn.rs` and `crates/aphrodite/Source/Hook/Terminal/Entry/Fn.rs` - 3 exports each, both entry points, both in B3's scope. They are the two public entry functions plus one helper each; the leaf convention says one export per file, so these are a real (if small) violation.
- `crates/aphrodite/Source/Flow/Engine/Offload/Fn.rs` - 3, also B3.
- `crates/aphrodite/Source/ABI/Handle/Lookup.rs` - 3, B1's scope, and the ABI tree's own "one export per file" rule is stated in the repository map.

## The regression class the previous sweep introduced

```sh
grep -rn 'let _ = .*\.then(|| [A-Za-z_]*\.\(push\|insert\|extend\|clear\)' crates/*/Source --include=*.rs
```

6 sites, unchanged from the dispatch state:

- `crates/aphrodite/Source/ChainSplit/Marked/Output/Fn.rs:29` and `:44` - the card's own worked example.
- `crates/aphrodite/Source/ChainSplit/Split/Close/Fn.rs:16`
- `crates/aphrodite/Source/ChainSplit/Split/Fn.rs:46`
- `crates/aphrodite/Source/Directive/Verb/Push/Fn.rs:9`
- `crates/aphrodite/Source/Retrieve/Page/Slice/Fn.rs:58`

Each is a mutation smuggled through a combinator: a closure is allocated, an `Option` is produced and discarded, and the write is hidden inside an expression so the source contains no `if`. This card repaired the two instances the build forced it to touch - `Debug/Log/At/Fn.rs` (whose stray `});` was the shadow of a removed closure) and `Debug/Warn/Fn.rs` and `Configuration/Template/Seeded/Fn.rs` (a side effect through `then`) - and left the other six alone, because reverting them is a card's redesign, not a mechanical fix. They are named here as the sweep's largest open item.

## The `if` count

Same probe as the research document's §7, applied to `crates/*/Source`:

```sh
grep -rnE '(^|[^A-Za-z0-9_])if[ (]' crates/*/Source --include=*.rs | grep -vE '^[^:]+:[0-9]+:\s*//' | wc -l
```

**163**, against 107 at this card's dispatch and 56 after the previous verification. The sweep raised it by 56, and this card raised it by 2 more - both of them conversions of a `then` combinator into the `if` the card asked for, which is the direction the card wants.

The files that gained `if`, largest first, with the classification the card asks for. All of them are genuine decisions; none is an unwound combinator - an unwound combinator would have shown up in the `let _ = ... .then(|| ...)` grep above, and the gainers are not in it.

- `crates/aphrodite/Source/Setup/MacOS.rs` 0→5 - guard clauses for the four platform steps (`ditto`, `xattr`, `install_name_tool`, `codesign`) and one `if let Some(name) = DylibIDName`. Each is a real boolean about a subprocess exit.
- `crates/aphrodite/Source/Preview/Builder/HTML.rs` 0→4 - one `if Count > 0` per summary part (headings, links, images, scripts), which is a presence test on a value.
- `crates/aphrodite/Source/Stage2/Reduce/Build/Fn.rs` 0→3 - `!Summaries.IsEmpty()` and its two siblings: presence of an accumulated collection.
- `crates/aphrodite/Source/Debug/Log/At/Fn.rs` 0→3 - the level-0 early return, the seek guard, and the fresh-line test. All three are the early-return validation the document allows.
- `crates/aphrodite/Source/Boot/Listen/Bind/Database/Fn.rs` 0→3 - two path-resolution branches and one `if let` on an optional parent.
- `crates/aphrodite/Source/Setup/Plan/Fn.rs` 0→2 - two independent booleans that decide whether a step is planned.
- `crates/aphrodite/Source/Flow/TurnContext/Budget/Fn.rs` 0→2 - the budget test and the oversized result.
- `crates/aphrodite/Source/Boot/Listen/Watcher/Debounce/Fn.rs` 0→2 - the settle deadline.
- `crates/aphrodite-hermes/Source/Debug/Warn/Fn.rs` 0→2 - the stderr fallback decision, converted here from a `then`.
- `crates/aphrodite/Source/Configuration/Proxy/Multi/Resolve/Fn.rs` 2→4 - one clamped-value warning; the other two counted lines are doc prose the probe's comment filter does not catch because they are indented.

The remaining 27 files gained one `if` each. A rise is not automatically wrong, and this one is explained: the sweep replaced `then`/`filter`/`map_or_else` spellings with plain decisions in the leaves it rewrote. What the rise does show is that the previous sweep's 56 was achieved partly by moving branches into combinators - which is the class named above.

## Verdict per card

The criterion is stated so the verdicts are readable:

- **CLOSED** - every seed the card names is implemented, the evidence its batch table requires exists and passes, and its files needed no change to their design.
- **PARTIAL** - the design landed but a required piece is missing: a forbidden shape it was told to remove is still present, evidence it was required to produce does not exist, or a conversion was left half-applied across its own tree.
- **OPEN** - the change is absent or contradicts the brief.

**B1 - ABI, guard, marker, logging: CLOSED.** All four seeds are real in the tree: 22 of 24 exports are `unsafe extern "C"` with `# Safety` contracts and the crate-wide `not_unsafe_ptr_arg_deref` allow is gone; the panic-payload disposition is one `Boundary::Panic` leaf used by both catch sites; `Guard::Cstr` answers `Enum::{Absent,Text(Cow),Oversize}`; and the three marker globals are one `Registration` value. The FFI contract checker passes with 0 violations, the generated bindings match, and the marker grammar renders correctly under it - the five marker-format tests that failed at the first test run were failing because a sibling test leaked a template grammar, not because of this card. Two of its own files did not compile (`Registration/With/Fn.rs`'s `super::Const` and the `Cap` leaf path), which is a path error rather than an unfinished conversion.

**B2 - proxy pipeline: CLOSED.** The cache identity is a value (`Cache::Identity::Struct`) with the digest as an index and the canonical body as equality evidence; content and encoded-body savings are separate measurements; the rewrite plan is consumed into the body. Its only build damage was a visibility (`Cache::Identity` was `pub(crate)` behind a `pub` field) and one needless deref, both fixed. Its one surviving forbidden shape is the `filter(|_| ...)` at `Proxy/Handle/Proxy/Respond/Buffered/Fn.rs:167`; it is recorded under the sweep rather than as a verdict downgrade because the card's batch table never names that shape - the sweep card does.

**B3 - hook, chain split, flow: PARTIAL.** The ownership chain is real: `Marked::Output` returns owned `Part` values that `Prepared::Segment` consumes, the prepared batch has closed fields and derived totals, plans carry their operands, the chunk policy names its target, `Hook::Emit` is one leaf for both transform paths, and the budget's protected overflow is a result type. Three of its eight seeds sit in `State/Session` and are documented rather than closed, which the card says plainly. What downgrades it is the regression class: `ChainSplit/Marked/Output/Fn.rs:29` and `:44` are the document's own worked example of the `let _ = ... .then(|| Parts.push(..))` shape, and the card left them exactly as they were while rewriting the function around them. Four more sites sit in its `ChainSplit` and `Flow` trees.

**B4 - analysis surface: PARTIAL.** The cap equality defect is fixed and measured, and its siblings (parameters at 35 bytes, signature at 60, floor, sample-long) went with it; the recognizer contract is one type over borrowed declarations with the category policy byte-identical. The preview-equality and cap-ownership proofs above pass on its tests. It is PARTIAL for two reasons: one production `filter(|_| ...)` survives at `Preview/Type/Fn.rs:39`, and the "barrier-controlled mixed-state regression" its batch table requires does not exist in the tree.

**B5 - startup and configuration: PARTIAL.** The split of `ApplyCompression` from `ApplyReload`, the one `Rendering` value and one `Publish`, the watcher's resolved target and bounded channel, and `Setup`'s `Ownership` and `Failure` values are all in place, and the publication-once property is what proof 3 rests on. It is PARTIAL because the `Rendering` value imported a proxy-gated type unconditionally - the hermes-side build without `proxy` did not compile - and because its own global-publication test raced the marker tests: the test published a template grammar and restored it at the end, holding no guard while doing so, so every concurrently running marker or preview test could observe the template mid-run. That race was the cause of 15 of the 15 test failures at the first test run. The guard is now a three-lock `GlobalsGuards` handle held for the whole test.

**B6 - runtime state: PARTIAL.** Five of its six seeds are closed with proofs: `Applied` is `Progressed | Completed(Finished)` (proof 6), the identity newtypes carry three `compile_fail` doctests, poll output is a `Cow` whose envelope arm moves the string, `BgStatus::Done`/`Failed` own their stored output with `Failed` holding a `NonZeroI32`, prefetch is Read → Prepare → Apply → Report, and the renderers take the fields they read. The accumulator cap and one-exit-code-one-task (proof 5) hold. It is PARTIAL on three counts: its tree contained the programme's only parse error (`Debug/Log/At/Fn.rs`), two values it introduced were never read (`Prefetch::Prepared::SourceSize` and the `Progressed` subject), and its handoff claims "prefetch reports `Loaded.Size` as bytes read; stat size kept as `SourceSize`" - a separation that never reached an output, because the apply step discarded it. The separation is now carried only where it is used.

**B7 - hermes tool and hook surface: CLOSED.** The `filter(|_| Object.len() <= 1)` matcher regression is a shared `Matcher/Sole`; the search envelope is `Payload::{Rows,Total}` behind the cheap `total_count` guard; the registry's retrieve and test siblings are `Target::{Path,Hash}` + `Narrow` and `Const::SAMPLES` + `Mode`; the directive set is typed `Entry` values with a per-file outcome; fail-open is narrowed to `(&Name, &Input)`; every hook arm and post-tool item is an `Fn` leaf. It needed four mechanical repairs - the `Choice::Enum` alias, the `Payload::Enum` and `Mode::Enum` leaf paths, three `.or_else(|| X::Fn::Fn())` closures that were redundant, and one genuine logic error in `Tool/Unwrap/Search/Rows/Text/Fn.rs`, which returned the unwrapped `Path` where the fold accumulator holds `Option<&str>` - and after them its tests pass.

**B8 - retrieve, resolve, directive, scripting: PARTIAL.** The loaded-versus-complete split, the per-expansion traversal that owns its path stack and budget, occurrences as borrowed spans, construction charged against the budget, the named grammar-tolerance policy, and the measured retrieval source are all in the tree, with nine new tests. It is PARTIAL because one instance of the regression class sits in its scope - `Retrieve/Page/Slice/Fn.rs:58`, `let _ = (ReachesEnd && Content.ends_with('\n')).then(|| Text.push('\n'))` - and because its own handoff records that `Resolve::Recursive::Inline` has no production caller, so the bounded path it built is not on the path the hook uses.

**B9 - hermes core: PARTIAL.** The design is present and correct in intent: `Broadcast::Report` with a `Continue` policy that keeps watching when the sweep could not look (proof 2 runs on it), a named `Outcome::{Held,Wedged}` for the bounded lock, a three-state debug flag, a recorded logging ownership, value-passing probes instead of env mutation, and a build stamp. It is PARTIAL because its conversion was never type-checked against a consumer: all thirteen of the types it introduced - `Outcome`, `Report`, `Start`, `Ready`, `Event`, `Handle`, `Identity`, `Ownership`, `Flag::State`, `Target`, `Mode`, `Payload`, `Outcome::Enum` - were referenced one leaf short at roughly ninety sites in its own tree and its own tests, which is the single largest error class in the build. One smaller defect belongs to it too: `Guard/ToJSON/Fn.rs` used `c_char` with no import.

## Where two cards met

No two cards defined the same type incompatibly. Three couplings were real, and each is a place where one card consumed another's type:

- **B5 consumed B9's `Session::Watcher` types.** B5 edited `Session/Watcher/Apply/Fn.rs` and `Configuration/Template/Fn.rs` - both recorded in its handoff - and its `Configuration/Template/Seeded/Fn.rs` referenced `Start::Enum` and called `ApplyGlobals` inside a `then` closure. The type is B9's; the mis-reference and the combinator are B5's.
- **B5 consumed B2's feature boundary.** The `Rendering` value is the bridge between the engine's `Proxy::Multi` document and the hermes-side `ConfigurationLoader` document. B5 wrote the engine-side conversion into a module that the hermes build compiles, without the `proxy` gate the type requires.
- **B3 and B7 met on the hook pipeline.** B3 consolidated terminal and tool-result emission into one `Hook::Emit`; B7 rewrote the `Hook::Replacement::Choice` value that emit consumes. The types agree; the failure was a leaf path, not a contract.

## What is still open

- Six `let _ = ... .then(|| ... push/insert/extend)` sites, named above. They are the sweep's largest open item and the exact class this card was told to catch.
- Two production `filter(|_| ...)` predicates, at `Proxy/Handle/Proxy/Respond/Buffered/Fn.rs:167` and `Preview/Type/Fn.rs:39`.
- The barrier-controlled mixed-publication test that B4's and B5's batch tables require, and that proof 3 therefore rests on a structural argument instead.
- `Resolve::Recursive::Inline` has no production caller: the bounded expansion B8 built is not the path the hook takes.
- The `if` count is 163, well above the 56 the previous verification recorded. The rise is explained file by file above, but a count that can be satisfied by moving branches into combinators has now been demonstrated twice to be the wrong measure; the two greps that catch the resulting shapes are the measure that matters.
