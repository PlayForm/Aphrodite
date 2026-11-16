# Sweep B5 - startup, configuration, installation

Scope: `crates/aphrodite/Source/{Boot,Configuration,ConfigurationLoader,Setup,Home}/**` and `Main.rs`.

## The specification was missing

`.hermes/research/04-rewrite2-batches.txt` does not exist in this tree: `.hermes/research/` is an
empty directory and no file of that name exists anywhere under the repository or `~/.hermes`. The
worked examples the card points at (lines 588-1000 and 3612-4116) could not be read.

Everything below therefore applies the **principles stated in the card itself** to the code as it
stands, and the seeds were treated as the specification of what to fix. Where a principle needed a
concrete contract the card did not fix (the debounce semantics), the choice is stated and defended
rather than assumed.

## Seeds

- **A reload sweep whose every session lock times out returns the same `false` as "reload
  disabled".** The sweep lives in `crates/aphrodite-hermes/Source/Session/Broadcast`, which this
  card puts out of scope ("the hermes-side watcher and session registry belong to another card")
  and which B9 was actively rewriting while this card ran (new `Broadcast/Report`,
  `Watcher/Outcome`, `Watcher/Continue`, `Watcher/Ready` appear in `git status`). Not touched.
  B9's tri-state report is in the tree and closes this seed.
- **Reload performs directive discovery under each session lock and then undoes it.** Closed, in
  the scope where the authority lives: `ApplyCompression` is now the startup pass (all sections)
  and `ApplyReload` is the reload pass (`Startup/Const` minus `Directive`). The
  snapshot-and-restore in `aphrodite-hermes/Source/Configuration/Reload/Fn.rs` is deleted, because
  there is nothing to restore - the reload sequence has no `Directive` step at all.
- **Global rendering settings published inside the per-session callback.** Closed for the
  publisher and the placement: `ApplyGlobals()` publishes the whole rendering set in one call, and
  `Session/Watcher/Apply::Fn` calls it once per reload (success arm only) instead of once per
  session. The old `ApplyPreviews`/`ApplyTemplates` pair is deleted, so the partial-publication
  shape cannot be written down again.
- **Family and templates published by two setters taking the write lock separately.** Closed:
  `Apply::Global::Publish::Fn` publishes `(Templates, Family)` under ONE `Template::State` write
  lock, so a reader observes the previous pair or this one. `Template::Set::Fn::Fn` and
  `Template::Family::Set::Fn::Fn` are no longer called by any config path.
- **The debounce waited 500 ms from the first event and then drained.** Closed, contract chosen and
  documented: **trailing-edge quiet period, capped** - 500 ms of quiet, ceiling 2 s from the first
  event. Rationale in `Boot/Listen/Watcher/Debounce/Fn.rs`: a reload wants the _finished_ file, and
  a pure quiet period starves on a directory that never goes quiet, so the ceiling bounds it. The
  old contract reloaded in the middle of a slow multi-pass write.
- **The dirty channel was unbounded and carried `()`.** Closed: capacity **one**, `try_send`, and
  the payload is the resolved `Target` (file + directory). The applier now reads the path from the
  event instead of taking a second, parallel `Path:String` argument that nothing kept in step.

## Files changed

### ConfigurationLoader

- `Configuration/Apply/Startup/Const.rs` (new) - the ordered startup applier list, as data.
- `Configuration/Apply/Startup/Fn.rs` (new, moved from `Apply/Compression.rs`) -
  `ApplyCompression`: state only, all sections, directives included.
- `Configuration/Apply/Reload/Const.rs` (new) - the reload applier list: `Startup` minus
  `Directive`, which is the whole narrowed-authority fix.
- `Configuration/Apply/Reload/Fn.rs` (new) - `ApplyReload`: state only, no directive step.
- `Configuration/Apply/Compression.rs` (deleted) - the single 8-call function it held is now the
  two passes above.
- `Configuration/Apply/Global/Rendering/Struct.rs` (new) - the process-global rendering
  configuration as one value, with `From<&MultiConfiguration>` and `From<&Configuration>`. Four
  hand-written publication blocks (engine load, dylib init, dylib reload, proxy `/reload`) became
  one description plus one publisher; the engine and the dylib now build the same value from
  different document types.
- `Configuration/Apply/Global/Publish/Fn.rs` (new) - the only writer of the preview cap, the marker
  registration and the `(templates, family)` pair; the pair under one lock.
- `Configuration/Apply/Global/Fn.rs` (new) - `ApplyGlobals()`, the one-call entry.
- `Configuration/Apply/Global/{Preview,Template}.rs` (deleted) - the two partial publishers.
- `Configuration/Apply/Global/mod.rs`, `Configuration/Apply/mod.rs` - declarations and the module
  doc that explains the two passes and why `Global` is separate.
- `Configuration/Apply/Section/Template.rs` - no longer registers the marker grammar (a section
  applier reaching for a process global); it writes `State.Templates` and resolves the session
  injection, nothing else.
- `Test/Guard.rs` - `GuardedApplyGlobals` added; it takes the marker, template and preview guards in
  that fixed order, which is the only nesting any test performs. `GuardedApplyCompression`'s doc
  corrected: it guards the cwd/env the directive applier reads, not a global it writes.
- `Test/Apply/Preview.rs` - `TestApplyGlobalsWiresPreviewMaximumChars`, published through the guard.
- `Test/Apply/Template.rs` - `TestApplyGlobalsRegistersTemplatesAndFamily`: asserts the templates,
  the family AND the marker grammar from one publication, then restores by publishing an empty
  document.
- `Test/Apply/Prompt.rs` - the marker-grammar assertions moved to the publication test; this file
  asserts the two state fields its name is about, which is what `ApplyCompression` owns.

### Boot

- `Boot/Listen/Watcher/Target/Struct.rs` (new) - the watch target resolved once: canonicalised
  absolute file plus its directory. Replaces a `contains("aphrodite.toml")` substring test that also
  matched `.swp`/`.bak`/temp files and matched nothing when the loaded file had another name.
- `Boot/Listen/Watcher/Source/Fn.rs`, `Source/Struct.rs` - exact-path filter on `Modify | Create`
  (an atomic save arrives as one or the other depending on the backend), capacity-one channel
  carrying the target, `try_send`, and a `Result<_, String>` return so "why is hot reload off" has
  an answer.
- `Boot/Listen/Watcher/Debounce/Fn.rs`, `Debounce/Outcome/Enum.rs` (new) - quiet-period debounce with
  a ceiling; the outcome distinguishes `Settled` from `Closed` instead of returning `()`.
- `Boot/Listen/Watcher/Loop/Fn.rs` - no `Path` parameter (the event carries it); a burst that ends
  because the channel closed is still applied, then the loop ends.
- `Boot/Listen/Watcher/Apply/Fn.rs` - takes the target, loads `&Target.File` (no lossy string).
- `Boot/Listen/Watcher/Outcome/Enum.rs` (new) - `Watching(JoinHandle)` | `Inactive { Cause }`:
  readiness is a value the caller reads instead of a warning from a task nobody holds.
- `Boot/Listen/Watcher/Stop/Fn.rs` (new) - the watcher is aborted after the drain; the handle is no
  longer discarded at startup.
- `Boot/Listen/Watcher/Fn.rs` - builds the watch before spawning (so a failure is reported), returns
  the outcome.
- `Boot/Listen/Watcher/mod.rs` - the four layers plus the lifecycle value.
- `Boot/Run.rs` - keeps the outcome and stops the watcher after `Drain`.
- `Boot/Configuration/Load/Fn.rs` - four publication blocks replaced by one
  `Publish::Fn::Fn(&Rendering::from(&Configuration))`.
- `Boot/Fn.rs` - `.expect("tokio runtime")` became `.context("build the tokio runtime")?`; a host
  that refuses a thread pool is a startup failure, not an invariant.
- `Boot/Listen/Bind/Database/Fn.rs` - the `.then(...).flatten().map(...)` chain with a mutation
  inside `map` became straight-line `if`/`if let`, and the `create_dir_all` failure now names the
  directory.

### Configuration

- `Environment/Text/Fn.rs` (new), `Environment/mod.rs` - `EnvironmentText`: a non-empty env value,
  `None` when unset. An exported-but-empty `APHRODITE_API_URL=` used to win the chain with an empty
  string and produce a proxy pointed at no upstream.
- `Proxy/Multi/Resolve/Fn.rs` - `APIURL`, `Model`, `CCRDatabasePath`, `NotifyURL`, `NotifyKey` go
  through `EnvironmentText`; the default listen address is built (`SocketAddr::from`) instead of
  `"127.0.0.1:9797".parse().unwrap()`; the timeout clamp is an `if` rather than a `then` statement.
- `Proxy/Multi/Load/Fn.rs` - `Load(impl AsRef<Path>)`. The watcher holds a resolved `PathBuf`; a
  `&str` signature forced a lossy conversion, and a lossy conversion of a non-UTF-8 path is a
  different file - a reload that reads nothing and reports success. Every existing `&str` caller is
  unchanged.
- `CLI/Setup/Struct.rs` - the `From<Command>` impl is deleted. It was total over `Command`, so it
  invented a full setup argument set (ports included) for a `run` invocation that has no setup
  arguments; nothing in the tree used it.

### Setup

- `Plan/Ownership/Enum.rs`, `Plan/Ownership/Fn.rs` (new) - `Dev | DevStaleSymlink | Catalog |
Uninspectable { Cause }` replaces the `CatalogInstall`/`RemoveStaleSymlink` pair, which admitted a
  catalog install with a stale symlink to remove and could not say "the path is there and I could
  not tell what it is". A `symlink_metadata` failure other than `NotFound` is now `Uninspectable`
  (report-only) instead of reading as "not a catalog install", which was the one answer that let
  setup write into a directory it could not identify.
- `Plan/Struct.rs`, `Plan/Fn.rs`, `Plan/mod.rs` - carry the ownership value; the step list branches
  on it (`matches!` over the two dev variants) instead of on a bool.
- `Execute/Plugin/Directory/Fn.rs` - one match, four named branches, each saying what it did or why
  it did nothing. The `then(Prepare).transpose()` shape is gone.
- `Execute/Failure/Struct.rs` (new), `Execute/Fn.rs`, `Execute/mod.rs` - `Execute::Fn` returns
  `Result<Report, Failure>` and the failure carries the steps that completed. The old `try_for_each`
  dropped the partial report exactly when it mattered: the doc claimed a failed run reported its
  prefix, and `?` made that false.
- `Run/Entry.rs` - a failed run prints "N of M steps completed" and the step names before returning
  the error; a successful run prints the count instead of `let _ = Report;`. The catalog/stale-link
  printouts branch on the ownership value.
- `Artifact/Stage/Fn.rs` - the staging path is an `OsStr` argument. `Partial.to_str().unwrap_or(
"dylib")` downloaded to a file named `dylib` in the cwd for a non-UTF-8 path and then reported the
  staging path as the result.
- `Dylib/Checksum.rs` - the hash argument is the path itself. `Dest.to_str().unwrap_or("")` hashed
  the empty string, so a real artifact was reported as a checksum mismatch.
- `MacOS.rs` - every tool argument is the path itself (three `to_str().unwrap_or("")` sites removed,
  where `ditto` copied nothing and `codesign` "succeeded" against no file); the `.then(|| {...})`
  copy fallback and the `.map(|name| {...})` dylib treatment became `if`/`if let`.

## Files reviewed and unchanged

Each of these was read in full and meets the standard as it stands; the reason is the group's.

- **`Main.rs`** - the binary root is one delegating call; nothing to represent.
- **`Boot/mod.rs`, `Boot/Banner.rs`, `Boot/Request/{mod,Enum,Fn,Setup/Fn}.rs`** - the request is
  already a four-variant value and the precedence is the order of an `or_else` chain; the banner is
  a log line with compile-time values.
- **`Boot/State/{mod,Build,Listener}.rs`, `Boot/Worker/{mod,Count/Fn}.rs`,
  `Boot/Listen/{Drain,Serve,Shutdown,Spawn}.rs`, `Boot/Listen/Bind/{mod,All/Fn,One/Fn}.rs`** - bind
  before serve is enforced by `Bound` existing only after both steps; the drain's three outcomes are
  a `select!` over distinct futures; the spawn's handles are all joined.
- **`Boot/Configuration/{mod,Compression/Fn,Credential/Fn,Path/{Fn,Found/Enum,Resolved/Struct},
Proxies/{Fn,Assemble/Fn},Source/Enum,Load/Loaded/Struct}.rs`** - `Source::Enum` is already the sum
  type that replaced `(Option<Multi>, Option<CLI>, bool)` plus an `expect`; `Found::Enum` already
  replaces the boolean; `Credential` is a named precondition rather than a branch inside assembly.
- **`Boot/Route/{mod,Build,CCR,Chat,Health/{mod,Public,Upstream},Management,Metrics,Relay,Retrieve}.rs`**
    - the two layer applications are the security boundary and are applied once, in `Build`; each route
      leaf is one `Router`.
- **`Configuration/{mod,CLI/{mod,Command/Enum},Environment/{Bool/Fn,Warn/Fn},Proxy/**}`** - the
  document structs are the wire format and stay `Option`-per-knob; `EnvironmentBool` and
  `EnvironmentParseWarn` deliberately conflate absent with unusable and document why (a typo'd
  override degrades to the file value and the warn is the signal); both were left as documented
  contracts rather than split, which would have churned `Proxy/Threshold/Resolve` in another card's
  area for no behavioural gain.
- **`Configuration/Test/**`** - the resolve tests assert non-empty env overrides and the three hard
  errors; the empty-env rule is new behaviour with no test contradicting it.
- **`ConfigurationLoader/{mod,Warn/Fn}.rs`, `Configuration/{mod,Struct,Default,Load/**,
Resolve/**,Apply/Section/{mod,ChainSplit,Debug,Directive,Engine,Flow,Prompt,Runtime}}.rs`** -
  `Struct` already distinguishes "defaults because nothing is configured" from "defaults because the
  file is broken" via `ParseFailure`; the resolution chains are one family per file and the `or_else`
  shape is already the value form of the precedence; the section appliers write state only.
- **`ConfigurationLoader/Test/{mod,Load,Resolve,Warn}.rs`, `Test/Apply/{mod,Compression,Directive}.rs`**
    - unchanged; `GuardedApplyCompression` still serialises them.
- **`Home/**`** (`mod`, `Configuration/Fn`, `Database/Fn`, `Directive/Fn`, `Environment/{mod,
Guard/Fn,Path/Fn}`, `Hermes/{mod,Fn,Opt/Fn}`, `Plugin/Fn`, `Runtime/{mod,Fn,Opt/Fn}`, `Tilde/Fn`,
  `Test`) - the precedence chain is one decision in `Runtime/Opt/Fn`, the degraded `.` fallback and
  the honest `Option` form are split into two leaves per home, and empty-means-unset is decided once
  in `Environment/Path`. Nothing to change.
- **`Setup/{mod,Plan/Step/Enum,Execute/{mod,Binary/Fn,Configuration/Fn,Directory/Fn,Registration/Fn,
Report/Struct},Register/{mod,Fn,Outcome/Enum},Verify/{mod,Hermes,SelfHash},Write/**,
Dylib/{mod,Checksum,Copy/**,Download/Fn,TargetTriple/Const},Artifact/{mod,Staged,Verified,Prepared,
Published},Test/**}`** - the plan is already a value, registration already returns three named
  outcomes, the artifact pipeline is already four values in a fixed order, and every writer already
  sets an explicit mode.

## Behaviour changes, and why

1. **Globals are published once per reload, and not at all for a broken file.** Previously a reload
   published them once per reachable session - zero times when every lock was unavailable. A parse
   failure publishes nothing, so the previous cap/grammar/templates stay in force, matching the
   existing per-session rule.
2. **The debounce is now a quiet period (500 ms) with a 2 s ceiling.** A reload no longer fires 500 ms
   after the first event of a slow write; it fires after the writes stop, or at 2 s if they never do.
3. **The proxy's config watcher accepts `Create` events on the target path as well as `Modify`**, and
   matches the resolved path exactly instead of a filename substring. An atomic save that reports the
   destination as created now reloads; a swap file or a backup named `aphrodite.toml.*` no longer
   does.
4. **An exported-but-empty `APHRODITE_API_URL`/`APHRODITE_MODEL`/`APHRODITE_DB`/`APHRODITE_NOTIFY_*`
   no longer wins the precedence chain.** Empty now means unset, the rule `Home::Environment::Path`
   already applied to paths.
5. **`aphrodite setup` reports partial progress on failure** and prints "N of M steps completed" on
   success instead of discarding the report.
6. **A plugin directory that exists but cannot be inspected is report-only**, where it used to be
   treated as setup's to write.
7. **`AphroditeInit` now publishes the rendering globals** (`ApplyGlobals`) after applying the
   session state; without it the marker grammar would have stopped being registered on that path,
   because `ApplyCompression` no longer registers it.
8. **The config watcher is stopped explicitly after the drain** rather than left to process exit, and
   a watch that cannot start is reported to the caller instead of warned from inside a task.

## Cross-card edits and notes

The API change (deleting `ApplyPreviews`/`ApplyTemplates`, moving marker registration out of
`ApplyCompression`) forced four one-to-three-line call-site edits outside this card's scope. All are
recorded on the owning cards:

- `crates/aphrodite-hermes/Source/Configuration/Reload/Fn.rs` - `ApplyReload`, snapshot/restore
  deleted (B9).
- `crates/aphrodite-hermes/Source/Configuration/Template/Fn.rs`, then `Template/Seeded/Fn.rs` when
  B9 split that file mid-run - one `ApplyGlobals()` (B9).
- `crates/aphrodite-hermes/Source/Session/Watcher/Apply/Fn.rs` - `ApplyGlobals()` once, success arm
  only (B9; this is the line most likely to collide with B9's in-flight rewrite).
- `crates/aphrodite/Source/ABI/Initialise.rs` - `ApplyGlobals()` after `ApplyCompression` (B1).

Left for the owning cards, with the reasons:

- **B9**: `Session/Broadcast`'s sweep result (seed 1) and a _sweep-level_ bound - the per-session
  `lock_bounded` deadline bounds one acquisition, not the pass, so a hundred wedged sessions still
  cost a hundred deadlines. `Session/Watcher/Apply` is being rewritten while this card ran.
- **B2**: `Proxy/CCR/Reload/Fn.rs` still publishes the preview cap, the marker registration and the
  two template halves by hand, in three separate critical sections. It should call
  `Publish::Fn::Fn(&Rendering::from(&Configuration))` - one call, and the pair becomes atomic there
  too.
- **B1**: `Marker/Format/Registration/Set/Fn.rs`'s doc still attributes the registration to
  `apply_compression`; it is now `ApplyGlobals`.

## Verification

No build was run (the card forbids `cargo check`/`test`/`clippy`/`fmt`; the single build belongs to
the verification card). What was checked mechanically instead:

- an ad-hoc harness of **39 assertions over the changed code** (`hermes-verify-b5.py`, run from the
  system temp dir, not committed): all 39 hold. It asserts the claims this document makes, by
  parsing the sources - the reload list is `Startup` minus `Directive` in the same order; the pair is
  written inside one `.write()`; the channel is capacity one with `try_send` and a typed payload; the
  debounce uses `timeout` with a 500 ms window and a 2 s ceiling and never a fixed `sleep`; the
  watcher returns an outcome and `Run` stops it after the drain; `Execute` returns the failure with
  the completed prefix; no code path references a deleted item; no setup shell-out converts a path
  with `to_str().unwrap_or`. It strips comments and string literals first, so a doc sentence cannot
  satisfy a claim about code;
- every leaf in this card's scope has a `mod.rs` and is declared by its parent (0 undeclared);
- brace/paren/bracket balance over all 642 changed-or-new Rust files (0 unbalanced);
- `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` is clean, and no
  file under `crates/aphrodite/templates/` was touched.

**This is static evidence, not a compile.** Nothing here proves the tree type-checks, and the harness
cannot see a mismatched argument or a moved field.

Observed while running it, outside this card: `crates/aphrodite/Source/StructExtract/Format/Cap/Fn.rs`
exists with `Format/mod.rs` declaring `pub mod Cap`, but `Cap/mod.rs` does not exist yet - that is
B4's area mid-write, not a break in this card's tree.

Known risk for the build card: the hermes crate is being rewritten concurrently (B9). It split
`Configuration/Template/Fn.rs` into `Template/{Seeded,Plain}/Fn.rs` while this card ran and carried
the deleted `ApplyPreviews`/`ApplyTemplates` calls into `Seeded/Fn.rs`; that file now calls
`ApplyGlobals()`. If it is split again, the same substitution applies. `Template::Set::Fn::Fn` and
`Template::Family::Set::Fn::Fn` still exist and are still used by `Proxy/CCR/Reload` and by tests.
