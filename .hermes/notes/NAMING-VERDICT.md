# Naming verification verdict

A rename is the one change where a green build proves almost nothing: the compiler is
equally happy with `Limit` whether or not the TOML key still reads `preview_max_chars`.
This card therefore proves two separate things — the tree builds, and every external name
is byte-identical — and keeps them apart so neither is mistaken for the other.

Nine rename partitions were arbitrated: `B1` boundary, `B2` proxy, `B3` hook/flow,
`B4` analysis, `B5` startup/configuration, `B6` runtime state, `B7` content,
`B8` Hermes tool surface, `B9` Hermes core.

---

## The ladder

Every gate was run from the repository root with its exit code read before any output was
inspected. CI builds Linux; a green macOS clippy is not evidence for that leg.

- **`cargo ci-check` → 0.** `check --workspace --release`, the same argv CI runs.
- **`cargo test --workspace` → 0.** 674 passed, 0 failed, 1 ignored, across 10 test binaries.
- **`cargo ci-clippy` → 0.** `clippy -p aphrodite -p aphrodite-hermes -- -D warnings`.
- **`cargo ci-fmt` → 0.** `fmt --all -- --check`, after `cargo fmt --all` repaired eight files.
- **`python3 Maintain/check_ffi_contract.py` → 0.** `0 violation(s), 0 warning(s) -> PASS`.
- **`python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py` → 0.** 23 tests, `OK`.
- **`diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` → 0.** Identical.

`crates/aphrodite/templates/` and `plugins/` are unmodified: the shipped `aphrodite.toml`
and the generated loader were never touched by the rename pass.

---

## The first build

No rename card compiled anything, so this card owned the first build. It took five passes
and **117 compile errors**, every one mechanical — a stale path, name or arity, never a
decision:

- **28 unresolved import or module path** (`E0432`, `E0433`). A leaf moved (`MarkerEntry`
  became `State::Marker::Struct::Struct`) or a module gained a level
  (`Proxy::Request::Route::Chat::Path` became `Chat::Const::Path`). This was the largest
  class and the one the rename cards predicted first.
- **43 stale function or value name** (`E0425`). A collapsed export kept its old verb name
  at the call site: `RenderPrompt` → `Render::Fn::Fn`, `CatalogSummary` → `Catalog::Fn::Fn`,
  `OnSessionStart` → `Start::Fn::Fn`, `ShippedSessionInject` → `Prompt::Const::Shipped`,
  and six more of the same shape.
- **15 stale method name** (`E0599`). The `AphroditeState` methods lost their qualifier:
  `InlineStorePut` → `Put`, `InlineStoreGet` → `Get`, `RecordChainSplit` → `RecordSplit`,
  `NoteSplitRetrieval` → `NoteRetrieval`, `RecordToolEvent` → `RecordEvent`.
- **11 stale field name** (`E0609`, `E0559`, `E0560`). `UpstreamHealthCache` →
  `UpstreamHealth`, `InlineCCRHits` → `InlineHits`, `CacheCompressThreshold` →
  `CacheThreshold`, `CodeMultiplierX100` → `CodeMultiplier`, `TypeFilter` → `Filter`,
  `name` → `Name`.
- **10 constant used as a value** (`E0423`). `Maximum::Signature::Const` names the _module_;
  the value inside it is `Const::Const`. Two `use … as Maximum` aliases resolved to the
  module, so every `saturating_sub(Maximum)` failed.
- **3 type mismatch, 2 call-on-non-function, 1 refutable pattern, 1 parameter shadowing a
  static** (`E0308`, `E0618`, `E0005`, `E0530`). All of them are the same failure: the
  one-word rule made an identifier collide with something already in scope. A `const
Extension` shadowed a closure binding `Extension`; a parameter `Maximum` was read as a
  constant _pattern_; a parameter `Paths` shadowed the imported `Paths` function; a
  parameter `Parent` shadowed the `Chain::State::Parent` static.
- **2 duplicate name after collapse, 1 trait bound** (`E0428`, `E0255`, `E0277`). Two
  partitions independently collapsed two different items onto one word in one scope: a
  test helper `Finished` collided with a test `Finished`, and an imported
  `Prefix::Const::Slow` collided with a test `Slow`.
- **8 files unformatted.** `ci-fmt` failed after the tree compiled; `cargo fmt --all` was
  the whole repair.

Two of those 117 were in files no rename card owned
(`Directive/Materialize/Write/Fn.rs`, `Tool/Unwrap/Search/Payload/Resolve/Fn.rs`) — a
partition left them unformatted. They were repaired with the rest.

---

## Contract proof 1 — configuration keys

**The claim.** Every field of every `Deserialize` struct still resolves from the same TOML
key. A grep cannot prove this: the compiler is satisfied whether the pin exists or not, and
a dropped pin silently resolves the field to `None`.

**The proof has two halves.**

The first is a comparison of _effective_ keys between the pre-rename revision and the tree.
An effective key is the explicit `#[serde(rename = "…")]` when present, else the field's own
name — exactly what serde does. Comparing effective keys rather than pins is what makes a
field renamed without a pin visible: the pin count can grow while the effective set stays
identical, and only the effective set is the contract.

```
python3 .hermes/tmp/nv/toml_keys.py
[PASS] [defaults]    head=4  tree=4
[PASS] [proxies]     head=15 tree=15
[PASS] [compression] head=16 tree=16
[PASS] [previews]    head=4  tree=4
[PASS] [prompts]     head=3  tree=3
[PASS] [templates]   head=4  tree=4
[PASS] [root]        head=6  tree=6
```

52 keys, identical on both sides.

The second half is a runtime fixture: a document that sets every key to a distinctive
value, deserialised through the real types, with each value read back off the field. A key
that stopped resolving leaves its field `None` and fails the assertion, which a text
comparison could not catch.

```
CONFIG_KEYS_LANDED: 46
```

46 field-level assertions across the six tables plus the root table's own key set. Both
halves pass.

---

## Contract proof 2 — JSON wire keys

**The claim.** Any struct serialised onto the proxy wire emits the same keys.

Four `Serialize` structs are reachable from outside the crate and are the shapes a client
actually reads. Each was constructed and serialised at runtime; the expected key set is
derived from the _pre-rename_ revision's own `Serialize` struct, not copied from the
current source:

```
WIRE ccr_create_response: compressed_size,hash,marker_size,original_size,token_savings_ratio
WIRE relay_response:      async_call,error,result,success
WIRE retrieve_response:   content,error,found,live,source,truncated
WIRE marker:              ccr_type,center,hash,meta,preview,size,turn
```

```
python3 .hermes/tmp/nv/wire_keys.py .hermes/tmp/nv/harness-target/release/nv-harness
[PASS] ccr_create_response
[PASS] relay_response
[PASS] retrieve_response
[PASS] marker
```

Two wire structs are `pub(crate)` and cannot be constructed from outside
(`Proxy::CCR::Notification::Event::Struct`, `Retrieve::Live::Container::Struct`). Their
keys are covered by the same effective-key comparison instead: every field of both carries
an explicit pin, and no pin value differs between the two revisions.

---

## Contract proof 3 — CLI flags

**The claim.** `--help` is byte-identical to the pre-rename output for the binary and every
subcommand.

```
diff .hermes/tmp/help-root-before.txt  .hermes/tmp/nv/help-root-after.txt   → 0
diff .hermes/tmp/help-setup-before.txt .hermes/tmp/nv/help-setup-after.txt  → 0
```

Both are byte-identical. The before-images were captured from the release binary before the
rename pass touched `Configuration::CLI`.

Two things about the subcommand surface are worth naming, because they look like gaps and
are not:

- **`run` has no help of its own.** `aphrodite run --help` does not print help — it
  dispatches the proxy. That is pre-existing behaviour of the clap enum, not a rename
  effect: the command variants were renamed, the dispatch shape was not.
- **The flag set is unchanged in the short form too.** `-h` lists the same 20 flags as
  `--help`, and `--version` still prints `aphrodite v1.8.1`.

The one flag whose spelling deserves an explicit note is `--apiurl` (no hyphen). It reads
like a defect and is not: it is the pre-rename derived name, preserved by an explicit
`long = "apiurl"` pin. Changing it to `--api-url` would be a breaking CLI change, so the
pin is the correct outcome.

---

## Contract proof 4 — ABI symbols

**The claim.** `check_ffi_contract.py` passes and the symbol count matches.

The checker passes with `0 violation(s), 0 warning(s)`. The built dylibs were then read
directly rather than trusted:

- **`libaphrodite.dylib` exports 25 `Aphrodite*` symbols** — 22 hand-written, one per
  `ABI/` leaf, plus 3 from the `stateful!` macro (`AphroditeCompress`, `AphroditeTerminal`,
  `AphroditeTransform`).
- **`libaphrodite_hermes.dylib` exports 10 `aphrodite_hermes_*` symbols.**

**Both numbers disagree with this card's brief, and the brief is what is wrong.** The card
expects `24` core and `11` Hermes. The measured split is `25` and `10`: the total is `35`
either way, so one symbol was mis-attributed between the two dylibs when the card was
written. The `11`th Hermes hit in a source grep is a doc comment in
`crates/aphrodite-hermes/Source/ABI/mod.rs`, not an export.

The count is not the contract — the _names_ are. Extracted from both revisions, the symbol
multiset is identical (`32` from `no_mangle` sites on both sides, before the three
macro-generated names), and `crates/aphrodite/Source/ABI/Macro.rs` still forwards the
caller-supplied identifier verbatim.

---

## Contract proof 5 — the three sweeps

### No new abbreviation

```
grep -rnE '\b(Env|Cfg|Impl|Num|Dir|Msg|Ctx)\b' crates/*/Source --include=*.rs | wc -l
```

**34 matching lines, 38 occurrences — not the 0 the card expects.** The same probe over the
pre-rename revision returns 44 occurrences, so **this pass added no abbreviation and removed
six**:

- **`Dir` — 18 occurrences.** A directory. It is a word, not an abbreviation of a compound,
  and it lives in `aphrodite-hermes`'s directive materialiser.
- **`Env` — 11 occurrences.** One module name (`Configuration/Test/Resolve/Env.rs`) and ten
  prose mentions of environment variables.
- **`Impl` — 9 occurrences.** The Rust construct: a `Stage2` matcher variant that maps the
  literal `impl ` keyword, its module, and prose.

The card's zero is unreachable without renaming words that name their own domain, and
renaming them would be re-running a partition's naming pass — which this card forbids.

### No leftover compound

```
grep -rnE '\b[A-Z][a-z0-9]+([A-Z][a-z0-9]+){2,}\b' crates/*/Source --include=*.rs | wc -l
```

**733 matching lines, 774 occurrences, down from 2508.** The pass removed 1734. Of the 774
survivors, every one falls into a class the doctrine allows, and the classes are counted
rather than asserted:

- **226 `#[test]` function names.** A cargo report label, not an item path — there is no
  module for the qualifier to move into. Partitions diverged here (`B3` renamed its tests,
  `B4`/`B7` left theirs); the residue is the same either way.
- **86 std or dependency names.** `NonZeroUsize`, `AsyncReadExt`, `DefaultBodyLimit`,
  `SocketAddrV4`, `TestCaseError`, `InMemoryCcrStore`.
- **127 prose occurrences** inside `//` and `///` comments.
- **335 occurrences of 44 names** in code — and this is the only class that needs a
  justification, itemised in the partition verdicts below. `222` of them sit in `B6`'s own
  paths: the `AphroditeState` fields and the `State/Const.rs` caps.

Only **33 names (36 occurrences) are new** relative to the pre-rename revision: 32 test
functions whose qualifier moved into their path (`TestResolveOneInline` → `TestOneInline`)
and `OutOfRange`, a bound-pair failure `B6` documented as having no single word.

### No `pub trait`

```
grep -rn '^pub(\(crate\))? trait ' crates/*/Source --include=*.rs
```

**Zero, on both revisions.** The "each with ≥ 2 impls" clause is vacuous — there is no trait
in either crate to check.

---

## Configuration keys that changed

**None.**

One thing looks like a change and is not: `Configuration::Proxy::Proxy::Struct`'s first field
became `Name` and gained `#[serde(rename = "name")]`. The _pinned_ key count therefore rose
by one (92 → 93) while the _effective_ key set did not move — the field was already read
from `name`, and it still is. This is the case the effective-key comparison exists to
distinguish, and it is the only field in either crate where a pin was added.

The same holds for the two flag surfaces: `clap` gained 18 `long` pins and 14 `value_name`
pins, all of them restoring a name clap would otherwise have re-derived from the new field
name. `--apiurl`, `--api-key`, `--maximum-context` and every `<VALUE>` are unchanged.

---

## Verdict per partition

**`B2` proxy — CLOSED.** Zero unjustified compounds, zero `#[test]` residue, no serde pin
added, no clap flag, no ABI symbol. Its one path change
(`Threshold::Const::ChatCompletionsPath` → `Request::Route::Chat::Const::Path`) broke five
consumer imports, all repaired here.

**`B4` analysis — CLOSED.** Zero unjustified compounds. 76 `#[test]` names remain by
design. No serde pin, no clap, no `extern "C"`.

**`B1` boundary — CLOSED.** 14 occurrences remain: `TransformTerminalOutput` and
`TransformToolResult` are the hook names Hermes dispatches on — an external wire contract
that must not move — and `ContextEngineEnabled` / `EngineThresholdPct` are `B6`'s fields
read from `B1`'s paths.

**`B3` hook/flow — CLOSED.** All 43 remaining occurrences are `AphroditeState` or `BgTask`
fields owned by `B6`. Zero residue of its own.

**`B5` startup/configuration — CLOSED.** 28 occurrences: 26 are `B6`'s state fields, and
`RequireMgmtToken` is an import alias that preserves the original middleware name for
`Boot::Route::Build`'s call sites. Zero residue of its own.

**`B7` content — CLOSED.** 11 occurrences, both `B6` fields (`ManualDirectiveTurn`,
`ExpiresAfterTurn`). Zero residue of its own.

**`B8` Hermes tool surface — CLOSED.** 3 occurrences, all core-crate `AphroditeState` fields
read across the FFI boundary. Zero residue of its own.

**`B9` Hermes core — CLOSED.** Shares the Hermes crate with `B8`; no unjustified compound of
its own survives, and its 10 ABI symbols are byte-identical.

**`B6` runtime state — PARTIAL.** 222 occurrences across 34 names, and they are the honest
remainder of the doctrine's reach, not missed work:

- **`AphroditeState`'s 45 fields** (~190 occurrences). A field has no path segment to carry
  a dropped qualifier. One-word field names would delete the distinction between six
  domains' overlapping words, so the fix is per-domain sub-structs — a structural rewrite,
  not a rename.
- **`State/Const.rs`'s caps** (`RecentMarkersCap`, `ToolEventCap`, `ReferencedFilesCap`,
  `SplitEventCap`, `SplitAdaptMinimumEvents`, `DefaultInlineByteBudget`). Six structures'
  caps co-located in one file; the fix is to move each constant into its owner's directory
  as `Const::Cap` — a file move plus a policy split.
- **Three documented collisions** kept with their qualifier because one word would be
  ambiguous: `OutOfRange`, `ObservedAtUnix`, `ArrivedAtUnix` / `LiveArrivedAtUnix`.

No partition is `OPEN`: nothing remains that a rename could still reach.

---

## Residue outside every partition

Two helper functions in `crates/aphrodite/tests/` carry three tokens and belong to no
partition — `CollectIncludeStrPaths` and `GoSourceWithErrorReturns`. They are integration
test scaffolding, not product identifiers, and were left alone rather than renamed into a
card that does not own them.

Six import aliases introduced by this card's repairs are two-token and so invisible to the
acceptance probe: `Struct as MarkerEntry`, `Struct as TemplatesConfiguration`,
`Struct as ActiveDirective`, `Struct as ToolEvent`, `Fn as CCRMarker`, `Fn as Substitute`.
Each keeps a consumer file compiling without re-running another partition's naming pass,
which is the constraint this card was given.

---

## How to re-run every proof

```
cargo ci-check; echo "EXIT:$?"
cargo test --workspace; echo "EXIT:$?"
cargo ci-clippy; echo "EXIT:$?"
cargo ci-fmt; echo "EXIT:$?"
python3 Maintain/check_ffi_contract.py; echo "EXIT:$?"
python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py; echo "EXIT:$?"
diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py; echo "EXIT:$?"
python3 .hermes/tmp/nv/toml_keys.py; echo "EXIT:$?"
python3 .hermes/tmp/nv/wire_keys.py .hermes/tmp/nv/harness-target/release/nv-harness; echo "EXIT:$?"
python3 .hermes/tmp/nv/sweeps.py; echo "EXIT:$?"
python3 .hermes/tmp/nv/classify.py; echo "EXIT:$?"
```

The config and wire probes are a detached cargo project at `.hermes/tmp/nv/harness/`; build
it with `cargo run --release` from that directory and it prints the key sets the comparison
scripts consume. Both `.hermes/tmp/` and the harness target directory are gitignored, so
none of this is product code.
