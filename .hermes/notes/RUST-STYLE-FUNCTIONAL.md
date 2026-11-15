# Rust functional house style

## 1. Authority

This document owns control flow, representation, laziness, ownership at effect boundaries, and
the naming of new helpers. It defers to `.hermes/notes/HOUSE-STYLE-REFACTOR.md` for where code
lives and how it is spelled: §4.1 (identifiers), §4.2 (no `pub use`), §4.4 (`anyhow`), §4.5
(`tracing` + `EnvFilter`), §4.6 (parallelism) and §4.7 (vertical rhythm) stand unchanged and are
cited here rather than restated - a worker reads both documents, applies that one for naming and
layout and this one for control flow, and where they touch the same line **this document wins on
control flow and representation while that one wins on naming and layout**. One clause of it is
superseded outright: **§4.3 is replaced by this document**. It required `match` for every `if`,
`.for_each` for every loop and a guarded `match` as the only conversion, which produced dummy
scrutinees, boolean matches with no-op arms and `Err(())` traversal sentinels in the tree; the
rule below is what replaces it. Two of its consequences are therefore revoked as well: the
crate-level `#![allow(clippy::match_bool)]` and `#![allow(clippy::single_match)]` in
`crates/aphrodite/Source/Library.rs` and `crates/aphrodite-hermes/Source/Library.rs` exist only
to stop clippy arguing for the spelling §4.3 banned, and the change that applies this style
removes them so the lints can enforce §3. Every other allow in those two files stands - see §9.

## 2. The rule

> Convert control flow into meaningful values and lazy composition. Prefer `Option` for absence,
> `Result` for failure, domain enums for decisions, iterators for sequences, `ControlFlow` for
> non-error termination. Use `match` to interpret those values. Preserve evaluation order,
> ownership, lock lifetimes and enclosing-function exits. Do not precompute alternatives merely
> to pattern-match them.

The distinction that makes the rule usable is **pattern matching is for structural alternatives;
predicate testing is not**. A `match` is right when the value's variants decide the branches: an
enum whose arms are its cases, an `Option` whose `None` is a real absence, a `Result` whose `Err`
is a real failure. A sequence of ordered rejection rules is not a decomposition of a value - it is
a policy about the input, and it stays as ordered guards, or it is lifted into a typed
representation and then matched. `if` is not a lesser form of `match`; it is the correct form for
a predicate, and `then`/`then_some`/`or_else` are the correct form when the predicate has to
produce a value.

The failure mode this rule exists to prevent is a rewrite that moves a branch without removing
it. Counting eliminated `if` tokens measures the rewrite, not the code: a `match ()` with four
guards has four branches, one scrutinee that carries no information, and less clarity than the
`if` chain it replaced.

## 3. Forbidden shapes

Each shape below appears in the tree today; each is small enough to check by eye, and two of them
are clippy-detectable once the §4.3 allows are gone.

### A dummy scrutinee

```rust
// Before - crates/aphrodite/Source/ABI/Guard.rs, `CheckContent`
match () {
	_ if Content.is_empty() => anyhow::bail!("empty content"),
	_ if Content.len() > MaximumContent => anyhow::bail!("content exceeds 16MB limit"),
	_ if Content.contains('\0') => anyhow::bail!("content contains null bytes"),
	_ => Ok(()),
}
```

```rust
// After - three ordered rejections, each error built only when it rejects
crate::Guard::Reject::Fn::Fn(Content.is_empty(), || anyhow::anyhow!("empty content"))?;
crate::Guard::Reject::Fn::Fn(Content.len() > MaximumContent, || anyhow::anyhow!("content exceeds 16MB limit"))?;
crate::Guard::Reject::Fn::Fn(Content.contains('\0'), || anyhow::anyhow!("content contains null bytes"))
```

Restoring one plain `if` per rule is not the fix either: it puts the predicate back at the call site,
and a single `if Content.is_empty() { bail!() }` - which is what that produces - silently drops the
second and third refusal. The predicate belongs in one primitive (§8) and never at the consumer.

A dummy scrutinee moves branching into the arm guards; it does not remove it. The `match` keyword
promises a decomposition of a value and delivers a predicate chain, so the reader looks for a
value that is not there. `match true` and `match false` are the same defect with the constant
spelled out. `Cstr` in the same file shows the other half of the cost: `match Ptr.is_null() { true
=> return None, false => () }` is a boolean match with a no-op arm, when `NonNull::new` would have
turned the pointer itself into the value worth interpreting (§4).

### A boolean `match` with a no-op arm

```rust
// Before - crates/aphrodite/Source/Boot/Listen/Watcher.rs
match Received {
	true => { /* reload */ },
	false => {},
}
```

```rust
// After
match Rx.recv().await {
	Some(()) => { /* reload */ },
	None => break,
}
```

The `false` arm exists only to satisfy exhaustiveness, and the boolean in front of it has already
discarded the information the arm needed: `Rx.recv().await.is_some()` turns channel closure into
`false`, so the false arm loops forever instead of terminating. A no-op arm is a signal that the
scrutinee was reduced too far - match the value the channel actually produced.

### An artificial error ending a successful traversal

```rust
// Before - crates/aphrodite-hermes/Source/Debug/Chain/Fn.rs
let _ = (0..16).try_for_each(|_| match map.get(&cur) {
	Some(p) if p != &cur => { /* push */ Ok::<(), ()>(()) },
	_ => Err(()),
});
```

```rust
// After
let Chain = std::iter::successors(Some(Session.to_owned()), |Current| {
	map.get(Current).filter(|Parent| Parent.as_str() != Current.as_str()).cloned()
})
.take(17);

let Chain:Vec<String> = Chain.collect();
```

"No parent" is completion, not failure, and the discarded `let _ =` proves nobody consumes the
error. `Err(())` in a traversal makes the type say a walk failed when the walk succeeded, forces
every caller to learn which error means "done", and hides the real failure channel the function
would need if it ever gained one. When a traversal genuinely needs per-step effects, terminate it
with `ControlFlow::Break`, whose payload can carry a reason that is not an error.

### A guarded `match` that restates a single `if`

```rust
// Before - crates/aphrodite-hermes/Source/Debug/Session/Record/Fn.rs
match parent {
	p if p.is_empty() || p == session => { /* publish, return */ },
	_ => {},
}
```

```rust
// After - the pair becomes a value, and the caller matches the value
// crates/aphrodite-hermes/Source/Debug/Session/Relation/Enum.rs
pub(crate) enum Enum<'a> {
	Root,
	Child { Parent: &'a str },
}

// crates/aphrodite-hermes/Source/Debug/Session/Relation/Classify/Fn.rs
pub(crate) fn Fn<'a>(Session:&str, Parent:&'a str) -> Enum<'a> {
	(!Parent.is_empty())
		.then_some(Parent)
		.filter(|Parent| *Parent != Session)
		.map_or(Enum::Root, |Parent| Enum::Child { Parent })
}
```

```rust
// the consumer - a structural match on a domain value, with no guard
match Relation::Classify::Fn::Fn(session, parent) {
	Relation::Enum::Root => aphrodite::Debug::Root::Note::Fn::Fn(session),
	Relation::Enum::Child { Parent } => Record::Fn::Fn(session, Parent),
}
```

When every pattern is a binding or a wildcard, the match contributes nothing and the guard carries
the whole decision: it is a predicate in a match's clothing. **The fix is not to write the `if`
back.** That restores the predicate at the call site - the exact thing being removed - and leaves
the decision as a boolean that nothing can pattern-match. The rule is to convert the boundary
condition into a value first: `Root` and `Child` decompose the pair, so the caller's `match` is
structural and the guard is gone. A boolean may survive exactly once, inside the primitive that
performs the conversion (§8), never in the consumer. Match guards remain legitimate inside a
genuine structural match - see §9.

### A tuple of precomputed alternatives matched afterwards

```rust
// Before - crates/aphrodite-hermes/Source/Tool/Unwrap/Fn.rs
match (obj.get("total_count").and_then(|c| c.as_u64()).is_some(), Search::Fn::Fn(obj)) {
	(true, Some((body, ct))) => Some((body, ct.to_string())),
	_ => None,
}
```

```rust
// After
obj.get("total_count").and_then(serde_json::Value::as_u64)?;

Search::Fn::Fn(obj)
```

The scrutinee is built by evaluating every alternative before the match selects one, so the loser
does its work for nothing - here the search normaliser runs on every envelope that is not a
search. A tuple of candidates is the opposite of a decomposition: it converts a sequence of
"try this, then that" into "compute all, then choose". Chain the alternatives with `?`,
`.or_else`, `.and_then` or `.find_map` so the next one runs only when the previous one yielded
nothing. The `.map(...)` that used to normalise the arm's type belongs to the next shape, not to
this one.

### A conversion that exists only so a combinator compiles

```rust
// Before - the arm allocates to match the chain's tuple type
Some((Body, Type)) => Some((Body, Type.to_string())),
```

```rust
// After - the chain's type follows the matcher it calls
pub(crate) fn Fn(content:&str) -> Option<(String, &'static str)>
```

The `.to_string()` converts `&'static str` into `String` only because the chain's return type is
`Option<(String, String)>`, and it allocates on every unwrap so that a tuple type can stay
uniform. The sibling `crates/aphrodite-hermes/Source/Tool/Unwrap/Search/Fn.rs` already returns
`Option<(String, &'static str)>`, so the borrowed type is the honest one: the arm passes the
matcher's value through unchanged, the chain returns the borrowed type, and the single conversion
happens where a caller genuinely needs ownership. The same applies to `.clone()`: a clone that
exists so a closure, an iterator chain or a `map` will type-check is a defect, and the fix is to
choose a signature or a representation that does not need the copy.

### A scalar carrying a decision the type system could carry

```rust
// Before - crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Fn.rs
) -> (anyhow::Result<reqwest::Response>, bool) {
	let mut UpstreamResult:anyhow::Result<reqwest::Response> = Err(anyhow::anyhow!("unreachable"));
```

```rust
// After
) -> Result<reqwest::Response, ForwardFailure> {
```

The `bool` says whether the last failure was a timeout, so `(Ok(Response), true)` is representable
and meaningless; a `ForwardFailure` with a kind carries the same fact without the illegal
combination, and the fabricated `"unreachable"` seed disappears because the terminal arms return
instead of overwriting a placeholder. The same reasoning covers a pair of booleans (three of the
four states are usually unreachable), a `String` that names one of a fixed set of cases, and a
`usize` whose `0` means "unlimited".

## 4. Representation before composition

Composition is easy once the value is right and impossible while it is wrong: most `if` chains
that resist conversion are resisting because the thing being branched on has no name. Choose the
representation first, then the combinators write themselves.

| Situation                                     | Representation                                                | Why                                                                                                                                     |
| :-------------------------------------------- | :------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------------------- |
| absence                                       | `Option<T>`                                                   | the type says "may not be there", so no caller can forget the branch                                                                    |
| nullable pointer                              | `Option<NonNull<T>>` via `NonNull::new`                       | converts nullability into a value before any dereference; it proves nothing about the pointee, so the `unsafe` block keeps its contract |
| failure                                       | `Result<T, E>` + `?`                                          | the `?` is the short circuit; an `if` that returns on error is the same flow spelled longer                                             |
| a set of alternatives                         | a domain enum                                                 | exhaustiveness turns "did I handle every case" into a compile error, and the variant names become the vocabulary of the module          |
| a sequence                                    | an iterator                                                   | laziness is the default; a `Vec` built to be walked once allocates to no purpose                                                        |
| early termination that is not an error        | `ControlFlow<B, C>`                                           | `Break` and `Continue` say which is which, where `Err(())` says the walk failed                                                         |
| borrowed-or-owned                             | `Cow<'a, str>`                                                | the unchanged case stays borrowed; `.into_owned()` is deferred to the point that actually needs an owned value                          |
| "unlimited or N"                              | `Option<NonZeroUsize>`                                        | `None` is unlimited, `Some(n)` cannot be zero, so the sentinel arithmetic disappears                                                    |
| a bounded accumulator                         | a struct that owns its invariant                              | the bound is checked at the only mutation point instead of at every call site                                                           |
| a completed validation step                   | a struct with private fields that only its producer can build | the existence of the value is the proof; a second check at the consumer is dead code                                                    |
| a shared payload                              | `Bytes`                                                       | clones share the allocation, so a body handed to a cache and to a response is copied once                                               |
| a decision only some branches need to compute | a closure                                                     | the work happens in the branch that asks for it, not before the branch is chosen                                                        |

The rule that keeps the table honest: **the representation must remove a decision, not rename
it**. Wrapping a `bool` in a two-variant enum whose only method is `is_true` removes nothing.
A struct with public fields that the consumer has to re-validate removes nothing. Ask what the
representation makes impossible; if the answer is "nothing", it is ceremony.

## 5. Laziness

A fallback is a promise that the lower-priority work happens only if the higher-priority work
failed. Precomputing the alternatives breaks that promise silently, because the result is
identical and only the cost and the side effects differ.

- `then(|| ...)` for a value that takes work to produce, `then_some(...)` only for a value already
  in hand. `Cond.then_some(Compute())` runs `Compute()` unconditionally.
- `or_else(|| ...)` for a fallback, never a tuple of precomputed candidates matched afterwards.
- `and_then` when the next step must not run after a failure; `.and(Next())` evaluates its
  argument eagerly, so the "next step" happens even when the previous one already failed.
- `?` is the short circuit. Every statement after a `?` is known to run only on success, which is
  why a validation sequence reads as a list of rules rather than a nest.
- A fallback closure must never run after a winner. Environment reads, path construction, file
  reads and `.await` all belong inside the closure of the branch that needs them.
- Seeding a chain with `None` - `None.or_else(|| First()).or_else(|| Second())` - makes the first
  real alternative look like a fallback and invites exactly this mistake; start the chain with
  the first alternative itself.

A laziness rewrite is a **behaviour claim, not a style claim**. A green compile proves the types
line up and proves nothing about evaluation order, so it is not evidence. The evidence is reading
the change and answering, in order: which expressions moved inside a closure, what did they
previously touch (allocation, filesystem, clock, lock, network), can the enclosing branch observe
that, and does the new order still reach the same branch for the same input. Where the answer is
not obvious from the diff, the change description states the evaluation order it now relies on.

## 6. Effect boundaries

Effects are ordered by what they touch, and the order is part of the contract: parse outside the
lock, prepare then apply, plan then execute, decode then transition then render. A function that
holds a lock while it parses, allocates, reads a file or awaits is doing two jobs and the second
one decides how long the first one blocks every other session.

The hard rule: **no lock-bearing helper is called while its mutex is already held**. The shape it
forbids is visible in `crates/aphrodite-hermes/Source/Debug/Session/Record/Fn.rs`, where a guard
on `SESSION_PARENTS` was held alive while calling `Chain::Fn::Fn(session)`, and in
`crates/aphrodite-hermes/Source/Debug/Chain/Fn.rs`, which locks that same
`SESSION_PARENTS:OnceLock<Mutex<HashMap<String, String>>>` declared in
`crates/aphrodite-hermes/Source/Debug/Chain/State.rs`. `std::sync::Mutex` is not reentrant: the
second `lock()` on the same thread does not fail, does not warn and does not return - it blocks
forever, and every other session that touches the map waits with it. The boundary, not a retry, is
the fix: `Record/Fn.rs` takes the guard in one scope, inserts the edge and resolves the root
through `Chain::Fn::chain_in(&map, session)`, which takes the map by reference and locks nothing,
while the locking wrapper stays available to callers that hold no guard of their own. Under the
naming rule of §8 those two halves are two leaves, not one file with two exports: the pure walk
belongs at `crates/aphrodite-hermes/Source/Debug/Chain/In/Fn.rs` (`Chain::In::Fn::Fn`) and the
wrapper keeps `Chain::Fn::Fn`.

Consequences worth stating because they are routinely mistaken:

- A `MutexGuard` that lives to the end of its scope is held to the end of its scope. Being unused
  after the last read is not a release; only dropping it or closing its block releases it.
- A guard held across an `.await` in an async function is a held lock across a suspension point,
  which is a deadlock risk in any runtime and a compile error only where the future must be `Send`.
- Nested locking that happens to work today is not safe; it works because the outer path never
  re-enters, and the next caller that does re-enter finds it.
- The boundary is per mutex, not per function: two different statics may be locked in sequence,
  never in a cycle, and the order must be the same everywhere or it is a deadlock waiting for the
  right interleaving.

## 7. Ownership

Ownership is a representation decision at the boundary where a value crosses from one owner to
another, and the default is to move rather than to copy.

- Consume at collection boundaries. A `Vec` that is built to be merged into a result is consumed
  by the merge (`into_iter`), not iterated by reference and cloned element by element.
- `std::mem::take` moves a field out and leaves its default behind, which is how a completion path
  hands its accumulated output onward without cloning it and without leaving the source in a
  half-valid state.
- `Cow` defers the decision. A transformation that usually returns its input unchanged returns
  `Cow::Borrowed` and allocates only in the branch that changed something.
- `Bytes` shares. A payload that is read by several consumers - a cache insert, a response body, a
  metrics length - is cloned as a handle, not as a buffer.
- `&'static str` stays borrowed. Converting it to `String` to satisfy a tuple, a struct field or a
  return type that never needed ownership is the same defect as a clone.

The counter-rule matters as much as the rule: **a clone introduced to satisfy a combinator is a
defect; a clone at a genuine fan-out boundary is not**. A value that must be held by two owners
for overlapping lifetimes is cloned once, at the point where the second owner appears, with a
comment naming that owner. A clone that exists because a closure captures by reference and the
borrow checker complained is a signal to change the closure's capture or the value's type.

## 8. How a new helper is named here

The review proposes helpers named `RejectWhen`, `MatchTerminal`, `BindOne`, `DecideRetry`,
`SelectEviction`, `CountedSample`. None of those names survive in this tree: a helper becomes a
leaf module whose directory carries the domain, whose file carries the kind, and whose export is
named by that kind. The call site writes the whole path, so there is no alias and no second name
for anything (§4.2 forbids the re-export that would create one).

### `RejectWhen` - a lazy rejection primitive

- Review name: `RejectWhen`.
- Module: `crates/aphrodite/Source/ABI/Guard/Reject/Fn.rs`, declared `pub mod Reject;` in
  `crates/aphrodite/Source/ABI/Guard/mod.rs` - **not** `Source/Guard/`. `Source/Guard/` is declared
  by `Source/Main.rs` (the binary), so library code cannot reach `crate::Guard`, and a primitive
  `CheckContent` must call has to live under a library module. Measured:
  `grep -rn 'mod Guard' crates/aphrodite/Source` lists `Main.rs:23` and no library declaration. A
  top-level `Reject/` leaf needs one line in `Library.rs`, which no package card owns - that move is
  a crate-root decision, not a package one.
- Item:

```rust
pub(crate) fn Fn<E>(Reject:Option<E>) -> Result<(), E> {
	Reject.map_or(Ok(()), Err)
}
```

- Consumer: `crates/aphrodite/Source/ABI/Guard.rs`, inside `CheckContent`, which writes
  `crate::ABI::Guard::Reject::Fn::Fn(Content.is_empty().then(|| anyhow::anyhow!("empty content")))?`
  - the crate-rooted path because the consumer's own module is also named `Guard`.

The predicate survives exactly once, here: `bool::then` turns the condition into an `Option` and
`map_or` turns its absence into `Ok`. `then` and not `then_some`, because `then_some(anyhow!("..."))`
would build the error on the accepted path too. A rejection sequence then reads as three `?`
statements in the original order, each constructing its error only when it rejects.

**Do not write `Cond.then(|| A()).unwrap_or_else(|| B())`.** `clippy::obfuscated_if_else` - part of
`clippy::all`, so it fails a `-D warnings` gate - rejects that form, and `then_some(A).unwrap_or(B)`
likewise. The lint-clean shapes that still remove the `if`:

| Shape | Lint-clean replacement |
| --- | --- |
| nullable or zero test | `Option` / `NonZeroU64` + `map_or` (the guarded work cannot run) |
| absence with a default | `Option::filter` + `unwrap_or_else` |
| a choice between two domain values | a domain enum matched structurally |
| a boolean gate in front of a block | `let _ = Cond.then(\|\| { ... });` - no `unwrap_or` follows, so no lint |
| a ternary over an `Option` | `match Opt { Some(_) => a, None => b }` |

`clippy::match_bool` is still allowed crate-wide, so `match Cond { true => ..., false => ... }` compiles
today - it is not a durable choice, because the allow is scheduled for removal.
`ABI::Guard` today documents itself as the management-route auth boundary; a general rejection
primitive widens that domain, so a second non-ABI consumer is the signal to move it to a top-level
`Reject/` leaf rather than to grow `ABI::Guard` further. That move needs one declaration line in
`Library.rs`, so it belongs to the crate-root owner, not to a package card.

### `MatchTerminal` - one envelope matcher

- Review name: `MatchTerminal`.
- Module: `crates/aphrodite-hermes/Source/Tool/Unwrap/Terminal/Fn.rs`, declared `pub mod Terminal;`
  in `crates/aphrodite-hermes/Source/Tool/Unwrap/mod.rs` beside the existing `pub mod Search;`.
- Item:

```rust
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<(String, String)>
```

- Consumer: `crates/aphrodite-hermes/Source/Tool/Unwrap/Fn.rs`, whose `terminal` closure becomes
  `Terminal::Fn::Fn(obj)` behind `use crate::Tool::Unwrap::Terminal;` - the same shape the chain
  already uses for `Search::Fn::Fn(obj)`.

`Unwrap/` is already a directory with `mod.rs`, `Fn.rs` and `Search/`, so this is an addition and
not a restructure: the chain keeps its position as the policy (`terminal().or_else(patch)...`)
and each arm becomes a named matcher that can be read, and checked, on its own. The review's name
for the returned tuple (`Extracted`) is not adopted - the tree's type is `(String, String)` and a
second name for a tuple is exactly the alias §4.2 forbids.

### `BindOne` - one listener

- Review name: `BindOne`.
- Module: `crates/aphrodite/Source/Boot/Listen/Bind/One/Fn.rs`, declared `pub mod One;` in
  `crates/aphrodite/Source/Boot/Listen/Bind/mod.rs`.
- Item:

```rust
pub(crate) async fn Fn(
	Name:String,
	mut CLI:CLI,
	Compression:Option<&CompressionConfiguration>,
) -> anyhow::Result<Bound>
```

- Consumer: the collection-level operation, which writes
  `Bind::One::Fn::Fn(Name, CLI, Compression)` inside the `futures::stream::iter(Proxies).then(...)`
  it already owns.

This one costs a restructure, and that cost is the honest part of the rule: the file
`crates/aphrodite/Source/Boot/Listen/Bind.rs` becomes `Bind/mod.rs`, and §4.2 forbids that facade
holding the orchestration function or re-exporting it. The collection-level operation therefore
moves to `crates/aphrodite/Source/Boot/Listen/Bind/All/Fn.rs`, and
`crates/aphrodite/Source/Boot/Run.rs` repoints
`Listen::Bind::Bind(Proxies, Compression.as_ref()).await?` to
`Listen::Bind::All::Fn::Fn(Proxies, Compression.as_ref()).await?`. The two leaves then read as
`Bind::All::Fn::Fn` and `Bind::One::Fn::Fn`, which is the domain and its cardinality, and neither
name needs a comment.

The same mapping without the worked call path: `DecideRetry` →
`crates/aphrodite/Source/Proxy/Handle/Proxy/Forward/Decide/Fn.rs` →
`Forward::Decide::Fn::Fn(...)`; `SelectEviction` →
`crates/aphrodite/Source/PollWorker/Insert/Select/Fn.rs` → `Insert::Select::Fn::Fn(...)`;
`CountedSample` → `crates/aphrodite/Source/Stage2/Reduce/Sample/Struct.rs` →
`Reduce::Sample::Struct::Struct`.

The rule the three worked examples prove: **the directory carries the domain, the file carries the
kind, the call site carries the whole path, and there is no alias**. Two corollaries follow from
it. A helper whose name would repeat its directory (`Reject/Reject/Fn.rs`) is named by kind
instead, because the repetition is the alias. A helper that needs a compound name to be
understood (`CountedSample`, `DecideRetry`) is telling you it has two domains, and the answer is
to nest once more, not to concatenate.

## 9. Where the rule stops

The rule governs code this repository owns the shape of. These are the places it does not reach,
and what each forbids changing.

- **The C ABI.** Exported symbols are the contract: cbindgen generates `_bindings.py` from them,
  the Hermes plugin calls them by name and `Maintain/check_ffi_contract.py` asserts them. This
  forbids renaming an export, changing its arity or its pointer types, and changing the JSON
  envelope a caller parses. `extern "C"` never returns `anyhow` - the boundary renders
  `format!("{e:#}")` into the error payload, so an `anyhow` conversion stops at the export's body
  and does not reach its signature.
- **Serde field names on wire types.** The relay request
  (`crates/aphrodite/Source/Proxy/Relay/Request/Struct.rs`) and the CCR notification
  (`crates/aphrodite/Source/Proxy/CCR/Notification/Struct.rs`) are serialised onto the proxy wire,
  as are the configuration tables; renaming a field changes the JSON a third-party client sees.
  This forbids renaming the field, and it also forbids "tidying" a `#[serde(rename)]` that exists
  to keep a wire name while the Rust name follows the house style.
- **Third-party API spellings.** `reqwest`, `axum`, `notify`, `serde_json` and `lru` name their own
  things, and a call site must use those names. This forbids renaming a method, a variant or a
  field of another crate, and it forbids wrapping one in a house-style alias instead - the wrapper
  is a second name for the same thing.
- **`#[cfg]` branches.** A `#[cfg(...)]` on an item, a statement or an arm is compile-time
  selection, not a runtime decision, and no representation makes it disappear. This forbids
  converting a `#[cfg]` into a runtime `match` on a constant.
- **Match guards.** A guard inside a genuine structural match is part of the pattern language and
  stays: `Some(p) if p != &cur` refines an arm that the variants already selected. This forbids
  reading §3's fourth shape as a ban on guards - the ban is on a match whose _only_ decision is
  the guard.
- **Const-context branches.** A branch inside a `const` item cannot use `?`, `Option` or an
  iterator chain. This forbids converting a const-context `if` into a combinator it cannot
  express, and it is the one place where a plain `if` is not a style choice.
- **The crate's existing clippy allow list.** In `crates/aphrodite/Source/Library.rs` and
  `crates/aphrodite-hermes/Source/Library.rs`, `#![allow(non_snake_case)]`,
  `#![allow(non_upper_case_globals)]` and `#![allow(clippy::not_unsafe_ptr_arg_deref)]` stand:
  the first two encode the house style itself, and the third is an FFI-boundary deferral that ends
  when the exports take `unsafe extern "C" fn` signatures. This forbids removing them as
  collateral of a style pass. The two that do not stand are `clippy::match_bool` and
  `clippy::single_match`, whose only reason was §4.3.

## Claim-to-test

Every claim below is settled by running the probe; a claim whose probe cannot be run is not
listed. "No output" means the command printed nothing and exited non-zero.

| Claim                                           | Probe                                                                                                     | Pass condition                                                                                            |
| :---------------------------------------------- | :-------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------- |
| No dummy scrutinee in the tree                  | `grep -rn 'match ()' crates/aphrodite/Source crates/aphrodite-hermes/Source`                              | no output                                                                                                 |
| No artificial error ends a traversal            | `grep -rnE -e 'Err\(\(\)\)' -e 'Ok::<\(\), \(\)>' crates/aphrodite/Source crates/aphrodite-hermes/Source` | no output                                                                                                 |
| No boolean match and no no-op arm               | `cargo clippy -p aphrodite -p aphrodite-hermes --all-targets -- -D warnings`                              | exit 0, with `clippy::match_bool` and `clippy::single_match` no longer allowed                            |
| No alternative is precomputed to be matched     | `grep -rnE 'match \(' crates/aphrodite/Source crates/aphrodite-hermes/Source`                             | every hit matches values already bound, with no call in the scrutinee that another arm would have skipped |
| No lock is held across a locking call           | `grep -rn -B3 '\.lock()' crates/aphrodite/Source crates/aphrodite-hermes/Source`                          | no enclosing function holds a guard on a static while calling a function that locks that same static      |
| No clone added for a combinator                 | remove the added `.clone()` and run `cargo check -p <package> --all-targets`                              | the resulting error is a move or borrow error, never a type mismatch in the combinator                    |
| Error messages unchanged unless the change says | `git diff -U0 -- crates` and read every changed `anyhow!`, `bail!`, `with_context` and `.context(` line   | each changed message string is named in the change description                                            |
| The C ABI contract is untouched                 | `python3 Maintain/check_ffi_contract.py`                                                                  | `PASS`, 0 violations                                                                                      |
| No facade re-export came back                   | `grep -rn 'pub use' crates/aphrodite/Source crates/aphrodite-hermes/Source`                               | no output                                                                                                 |
| Formatting is rustfmt-clean                     | `cargo fmt --all -- --check`                                                                              | exit 0                                                                                                    |
| Both crates still build                         | `cargo check -p aphrodite -p aphrodite-hermes --all-targets`                                              | exit 0                                                                                                    |
| Both crates still pass their tests              | `cargo test -p aphrodite` and `cargo test -p aphrodite-hermes`                                            | exit 0, 0 failed                                                                                          |
| No fallback runs after a winner                 | replace a lower-priority closure with `panic!()` and run `cargo test -p <package>`                        | no test panics, so the closure never runs once a higher-priority branch has won                           |
