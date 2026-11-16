# CFV - verification of the control-flow and construction pass

CF was forbidden to build, so this is the first compile of its changes. The
verdict is that the pass is sound: the whole ladder is green, the four counts
moved to where CF said they moved, and every break the compiler found was
mechanical - a path, a declaration, a type, a lint - never a decision. Nothing
in this run redesigned a single CF choice.

---

## The ladder

Every gate below ran on the tree CF handed over plus the repairs in this run.
Each exit code was read from the command itself, never from a piped or
truncated stream.

- **`cargo ci-check`** - exit **0**. The whole workspace type-checks in the
  release profile CI uses (`check --workspace --release`). The only output
  beyond that is the build script's standing note that the committed
  `_bindings.py` already matches the ABI.
- **`cargo test --workspace`** - exit **0**. **674 passed, 0 failed, 1
  ignored** across ten test targets: 558 lib, 17 binary, 83 bridge lib, 13
  integration (classifier 5, packaging 2, roundtrip 4, serde 2), 3 doc tests,
  and two targets that hold no tests. The one ignored test is the
  release-download checksum probe, ignored by design because it reaches the
  network.
- **`cargo ci-clippy`** - exit **0**. The alias runs `clippy` over both packages
  with `-D warnings` appended, so a clean exit means zero warnings in both
  crates, in lib and binary targets alike.
- **`cargo ci-fmt`** - exit **0**. `fmt --all -- --check`, zero diffs. It was
  **49 files** dirty on arrival; CF wrote the pass without running rustfmt.
- **`python3 Maintain/check_ffi_contract.py`** - exit **0**. 10 exports, 7
  required, 10 argtypes, **0 violations, 0 warnings**.
- **`python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py`** -
  exit **0**. 23 tests, OK. The generated bindings still describe the ABI the
  crates export.
- **drift guard** - exit **0**. `diff -q` over `plugins/aphrodite/__init__.py`
  and `crates/aphrodite/templates/__init__.py` reports the two files identical.

---

## The four counts

"Before" is the tree at the last commit, which is the pre-CF state: the whole
pass is uncommitted working-tree content. "After" is the tree that just passed
the ladder. Both sides were measured with the same probe, over
`crates/*/Source/**/*.rs` only.

- **`if` statement lines** - **134 → 0**. The probe is a line whose first token
  is `if` (after an optional closing brace), comments excluded.
- **`if` live lines** - **163 → 24**. Every non-comment line holding the token.
  Those 24 are **16 match guards and 8 prose strings**, read one by one; not
  one of them is a branch. `if let` went from **59 → 0** as well, which the
  card's four counts do not ask about but the same probe shows.
- **`for` loops** - **13 → 1**. The survivor is the declared `.await` residue.
- **`let _ = cond.then(|| side_effect)`** - **24 → 0**. Widening the probe to
  any argument (`let _ = cond.then(callback)`) leaves exactly **1**, and that
  one is a named primitive rather than a construction hack - see the residue.
- **`filter(|_| cond)`** - **5 → 1**, and the survivor is a doc comment
  describing the regression the idiom caused. In code: **4 → 0**.

Reconciliation with the dispatch numbers (if 143, for 13, then-hack 6, filter
5): `for` 13 and `filter` 5 reproduce **exactly**. The then-hack probe
reproduces **24**, not 6, on the pre-CF tree - the dispatch figure was a
subset, most likely the partitions dispatched together rather than the whole
tree. The `if` figure is the one number that does not reproduce at all: the
same tree measures 134 statement lines, 163 live lines and 246 token
occurrences, so 143 was neither a line count nor an occurrence count under any
probe tried here, and CF's own 141 was derived by adding diff-removed lines to
it. The number that decides the card is unambiguous either way: **zero `if`
statements remain, in either crate, in lib and binary targets**.

---

## What the first compile found

Twenty-two mechanical breaks across six rounds, each fixed at the site. They
are listed by class because the classes are the lesson: **none of them is a
decision CF made.**

- **Five leaves that no `mod.rs` declared.** `Walk::Build::Writer`,
  `Piece::Render`, `Entry::Line`, `Kind::Value` and `Label::Render` all existed
  as directories with real files inside, and their parent facade declared only
  the sibling. The files were written, the module was never wired, so the path
  resolved to nothing. Five one-line declarations fix it.
- **Three paths that named the wrong parent.** `Walk::Build::Writer` is under
  `Recursive`, not directly under `Resolve` (two files); `super::Select` in the
  hermes watcher's `Source` leaf meant the sibling, which does not exist, rather
  than `Session::Watcher::Select`; and `Metrics::Route(State)` became
  `Metrics::Route::Route(State)` once `Metrics` turned from a file into a
  directory and the function moved inside it.
- **Three shapes the compiler would not infer.** `Node::Read` indexed with
  `**Key` where `Key` is `&&str`, so the index was `str` and unsized - one
  deref, `*Key`. `Bucket::Struct::Struct::New()` named a constructor that does
  not exist, because the constructor is the leaf `Bucket::New::Fn::Fn()`.
  `Piece::Render::Fn::Fn` took `Piece`, the module, as a type, where the type
  is `Piece::Enum::Enum`.
- **Two bindings that shadowed a variant.** `|Done| Ok(Done)` bound the name
  `Done` inside the closure while `Outcome::Enum::Enum::Done` is a variant of
  the type being returned - `#[deny(bindings_with_variant_name)]`, and rightly.
  The closure is `Ok`, the variant constructor itself. In
  `ChainSplit::Marked::Output::Line::Fn` the catch-all arm was written `Text =>`
  and read as a variant that does not exist; it is `_ =>`, same arm, same
  value.
- **One closure that returned a value where the adapter wanted none.**
  `for_each(|()| UpdateFromPoll(..))` returns `bool` because the poll update
  reports whether it applied; the adapter discards, so the body is a block.
- **One visibility leak.** `Writer::Step::Fn::Fn` was `pub` over `pub(crate)`
  argument types - four `private_interfaces` warnings that `-D warnings` would
  have turned into errors. It is `pub(crate)`.
- **Four doc lines that read as markdown lists.** A line whose first character
  is a dash continues a list item rather than opening a paragraph, so clippy's
  `doc_lazy_continuation` demanded indentation. Both passages now carry their
  aside in parentheses, so the sentence survives and the list marker is gone.
- **Three lint gates that had to be answered, not dodged.** See the next
  section, because these are the only place the verification touched style.

`cargo fmt --all` rewrote 49 files. Formatting only - the diff is line breaks
inside chains, no token changed.

---

## The three lint gates

Clippy's `obfuscated_if_else` fires on exactly the vocabulary CF exists to
establish: `Cond.then_some(A).unwrap_or(B)` is, to the lint, an `if .. else ..`
written badly, and it wants the branch back. Eleven sites hit it. Re-branching
them would undo the pass, and silencing it file by file would hide the rule
where it matters, so the crate roots carry a documented gate instead:
`#![allow(clippy::obfuscated_if_else)]` in `aphrodite`'s `Library.rs` and
`Main.rs` (lib and binary are separate crate roots) and in
`aphrodite-hermes`'s `Library.rs`, each with the reason written above it. This
is the same shape the tree already uses for `non_snake_case`,
`non_upper_case_globals` and `upper_case_acronyms`: the house convention is the
contract, so the lint that contradicts it is switched off where the convention
is declared.

`clippy::single_match` fires on the shutdown signal registration, which is a
`match` on a `Result` whose `Err` arm is `()`. Clippy wants `if let` - the
branch form this pass removed from the tree - so that one site carries a local
`#[allow]` with the reason. It is one site, not a policy.

`clippy::redundant_closure` fired on the closure this run introduced while
fixing the shadowed variant; it is gone, replaced by the variant constructor.

---

## The residue

Every line below is either not control flow or is a language limit. Nothing in
this list was left by accident.

**Prose that contains the word "if" (8).** `hermes/Directive/Schema` and
`hermes/Schema/Retrieve` carry the tool-schema descriptions the model reads;
`hermes/Bin/SetupHelper` is a `println!` telling the operator what to run next;
`StructExtract/Test/Go` is an assertion message; `Flow/Prompt/Const` (2) and
`Flow/Test/Prompt` (2) are the prompt text and its expected copy. Rewording any
of them would change what the model reads or what the test asserts.

**Match guards inside a structural match (16).** `Proxy/State/Build` refines
the `Token` arm with `!CLI.NoCCRMarker`; `Proxy/Analysis/Classify/Envelope`
refines the parsed-JSON arm with a content test; `Marker/Format/Vars` is the
name-character class of the scan loop; `Catalog/Size/Classify` (2) is the
threshold ladder on `Bytes`; `Setup/Plan/Ownership`, `Setup/Dylib/Checksum`
(2), `Setup/Artifact/Stage` and `Setup/Verify/Hermes` each refine one arm of a
`Result` match; `hermes/Directive/Materialize/Write` (2) are byte-identical and
file-not-found, the two arms that decide a file's outcome; and
`PollWorker/Test/Update/{Attribution,Poll}` (3) with
`ConfigurationLoader/Test/Apply/Template` are `matches!(.. if ..)`, the guard
form of `matches!` in a test assertion.

**A `for` whose body awaits (1).** `Retrieve/Test/Handler` drives three
`retrieve` requests through the axum handler in one runtime block. An iterator
adapter cannot await, and this crate's `futures` dependency is optional, so the
loop stays and says so in a comment.

**A `filter(|_| ...)` that is prose (1).** `hermes/Tool/Unwrap/Matcher/Sole`
documents the regression the predicate-ignoring filter caused. It is the only
surviving hit of the idiom in the tree, and it is a warning, not a use.

**A `let _ =` gate around a callback (1).** `Proxy/Log/Dev/Fn.rs` is
`let _ = Dev.then(Line)`, and it is deliberately a primitive: `Line` is an
`impl FnOnce()` parameter, so `bool::then` keeps the closure - and any
`format!` or preview render inside it - unbuilt on the disabled path, at one
site instead of a `match` at every call site. The pass's own probe did not
count it because the argument is a parameter rather than an inline side effect;
it is named here so the count is not mistaken for zero by a wider probe.

**Documentation that mentions "if" (81 lines).** Not hits under the probe, but
stated so the 24 is not read as a smaller tree than it is: the crate documents
its own decisions in prose, and "if" is a word in that prose.

---

## Behaviour

The pass changed no answer for any input. Three items are worth naming, and
one of them was a defect found during CF's own review rather than by this run.

- **Deletion, not a change.** `Marker/Parse/Valid` had a `h.len() < 8` pre-gate
  no input could reach: an `i:`/`I:` prefix leaves at most `len - 2`
  characters, so a hash under 8 bytes already failed the 6-character floor the
  next test applies. The gate is gone and the reachable answers are identical.
- **`Flow/TurnContext/Budget::DropToBudget`** is now a search over the prefixes
  the drop loop walked. The measurement only shrinks as the prefix does, so the
  first fitting prefix is the one the loop stopped at, and "no prefix fits" is
  the overflow outcome rather than a second test on the bytes.
- **`Session/Watcher/Run`** composes the wake and the keep-watching policy into
  one `Step`, so the loop obeys one condition instead of two - same events,
  same applies, same exits.
- **The loopback guard.** CF's review caught a subagent inverting
  `ensure!(Addr.ip().is_loopback())` into `ensure!(!...)`, which would have
  rejected every loopback client and admitted every remote one. It is fixed in
  the tree, and this run proves the direction by test rather than by reading:
  `Guard::Test::Loopback::TestCheckLoopbackRequestRejectsNonLoopbackPeerRegardlessOfHost`
  and `...AllowsIpv6Loopback` both pass.
- **Everything this run touched** is a name, a path, a declaration, a type, a
  visibility or a formatting break. Two edits sit inside a body - the `Text`
  binding becoming `_`, and the absent counter gaining `.unwrap_or_default()` -
  and both keep the arm they replace: `_` matches what `Text` matched, and
  "absent member" already meant "no line written".

---

## Claims and the gate that answers them

- CF's changes compile in both crates, lib and binary targets: `cargo ci-check` exits 0.
- Nothing observable changed - `cargo test --workspace` 674 passed, 0 failed,
  the loopback, marker-parse, budget, watcher and resolver suites included.
- The style survives `-D warnings` - `cargo ci-clippy` exit 0.
- The tree is formatted - `cargo ci-fmt` exit 0.
- The C ABI and its generated bindings still agree with the source -
  `check_ffi_contract.py` PASS and 23/23 in `test_finalize_bindings.py`.
- The plugin and its template are one file - the drift guard is silent.
- The counts moved - the probes in "The four counts", measured on both trees
  with the same command.
