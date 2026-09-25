---
name: parallel-delegation-execution
description: "Use when running large repo tasks as parallel delegate waves on the Aphrodite monorepo (PlayForm/Aphrodite, Development branch)."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
category: engineering
category_taxonomy: engineering/parallel-delegation-execution
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, delegating, wave-planning, owning, verifying, steering]
        related_skills:
            - hermes-agent
            - plan
            - simplify-code
            - skill-library-refactoring
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
owns:
    - Wave planning and dispatch of delegate_task children
    - Disjoint file ownership allocation across parallel waves
    - Post-wave verification and integration passes
depends_on:
    - hermes-agent (delegate_task semantics, runtime cap)
    - aphrodite-orientation (preflight gate before any wave)
verification:
    source_of_truth:
        - .hermes/AGENTS.md (repo facts, branch conventions)
mutation_level: orchestration
---

# Parallel delegation execution (Aphrodite)

Running large multi-crate tasks on the Aphrodite monorepo
(PlayForm/Aphrodite, Development branch) - big renames, feature build-out,
issue-backlog sweeps, doc rewrites - as waves of parallel `delegate_task`
subagents instead of doing everything sequentially in the parent context.
Standing workflow: break large tasks into many small pieces, launch multiple
at a time (up to the runtime cap), keep progress in the repo planning archive
(`.hermes/notes/`), never /tmp.

## Always-on rules

- **Disjoint file ownership is the hard constraint.** Every child task names
  the exact files it may create/modify and forbids touching anything else
  ("this task owns crates/aphrodite/src/x.rs ONLY"). Two children editing the
  same file is the one failure mode that breaks a parallel wave. Never
  parallelize the shared files (workspace manifests, .gitmodules, directory
  renames) - do those yourself first, verify, then fan out. When two
  pre-planned tasks both need the same integration file (e.g. a CLI main.rs:
  one wants a new command, the other a new library module), split ownership
  instead of sequencing: one child owns the shared file and its wiring, the
  other exposes only the library API and is told explicitly "do NOT touch
  main.rs/lib.rs - the orchestrator wires the CLI after the wave lands"; you
  integrate the two between waves. Consumers of a new API (integration tests,
  fixtures, installers) go in a LATER wave than the API's creators - run in
  parallel they compile against the old tree and fail.
- **Children know nothing of this conversation.** Embed the full contract in
  each task: repo absolute path, house style (tabs, license, author), the
  required docs/instruction files to read, the exact type/API shapes to build
  against, and the verification command with its expected output.
  Self-contained tasks do not drift.
- **Batch 4-at-a-time (runtime cap).** `delegate_task(tasks=[...])` runs the
  batch concurrently; `delegation.max_concurrent_children` caps it. Do not
  poll transcripts to wait - the batch re-enters the conversation on
  completion. Steer (not stop) a child that drifts; steer text is queued to
  its next tool result. Check the child is still LIVE first
  (`delegate_task action=list`) - a completion that already landed cannot
  receive the steer (`missed_steer` in its completion report); fold a late
  scope-extension request into the orchestrator's integration pass instead
  of re-dispatching.
- **Pair-shaped waves are the standing default.** Dispatch exactly two
  children with disjoint file ownership for a typical phase - one owns the
  primary artifact + its consumer, the other owns the adjacent files/docs.
  Four is the runtime cap, not the default shape; do not fan out more than
  two unless the phase genuinely splits into more disjoint chunks (3-4
  one-file-owner surfaces - e.g. workflows / scripts+crates / root configs
  is a legitimate three-way wave when each surface is internally coherent).
- **Self-reports are not facts - and a child's "3x green" suite is not green
  in YOUR env.** A child's subprocess may NOT inherit env vars the parent
  exported earlier in the session (a probe set `APHRODITE_PREVIEW_MAX_CHARS`
  in the parent terminal; the child's runs stayed green while the parent's
  identical command failed 21-25 tests every run). A child reporting a clean
  suite run is a FALSE NEGATIVE until the parent re-runs the exact command in
  its own environment - an exported env var, not a code race, was the whole
  cause of the failures the child could not see. Conflicts between two
  children's reports about the same file resolve by reading the on-disk state.
- **Keep progress in the repo planning archive** (`.hermes/notes/` - the
  Aphrodite family convention; confirm with `git check-ignore -v` before
  choosing a directory, since creating the wrong dir leaves an unignored
  directory that pollutes git status and the auto-committer can sweep) as a
  plan file: checkboxes per phase, updated after each wave, so the next
  session resumes from the topmost unchecked task. Never /tmp. The archive
  may be gitignored by design (`git check-ignore -v <path>` confirms): then
  its edits never appear in `git status`/`git log` and the auto-committer
  never touches it - an empty diff is the expected state, not evidence of
  a lost write. A single long-running verification (CI release poll,
  installer e2e) is a valid one-child delegation: dispatch it, do not await
  it, and let its result re-enter when done. The child must run the poll as
  a BOUNDED FOREGROUND command (`gh run watch <id> --exit-status` with a
  generous timeout, falling back to a 30s `gh run view` poll loop) - never
  instruct it to leave a background process: a subagent-owned background
  process is KILLED when the child exits, its completion notification never
  reaches the parent, and the task returns "awaiting notification"
  incomplete with zero verification done.
- **Finish the sweep directly, never by more delegation.** Standing rule:
  once the parallel implementation waves land and are verified, the "proceed
  until everything is done" pass is executed by the orchestrator itself
  in-session (fixes, wiring, docs, CI, probes, archive updates) - no more
  workers/agents. Children are for parallel implementation; finishing
  touches are single-threaded by preference.
- **Relaunch a failed child - never hand-execute its scope mid-wave
  (standing orchestration preference).** When a child dies (401/429/timeout),
  re-dispatch it instead of writing its files yourself: the parent organizes
  and verifies, children implement. This governs scope a child already OWNS
  mid-wave; it does not repeal "finish the sweep directly", which still
  governs the final integration pass after the waves land.
- **Read the NEWEST planning/instruction file before executing a mass
  change.** Later directives supersede earlier ones and can reverse a rename
  or brand decision made earlier in the same session - a scripted mass
  rewrite is costly to undo.

## Wave structure

1. Parent does the shared-file foundation first (renames, manifests,
   .gitmodules, symlinks) and verifies (cargo test / smoke probe green).
2. Wave 1: new modules, one child per file, each with the exact public-API
   contract it must expose (other children will import those names).
3. Parent integrates: run the workspace test once after the wave; fix
   cross-module mismatches yourself (children can't see each other's code).
4. Wave N: dependent layers (CLI binary, tests, installers) - same rules.
5. Before declaring done, run the full verification ladder yourself and
   record results in the planning archive.

## Validate-then-fix issue waves (triage pairs)

For "check if these issues are valid and fixable" runs, split into two
phases per task, both pair-shaped:

1. **Parent establishes the repro baseline itself first** (cheap checks:
   run the failing test, stage the shallow path, trace the claimed code
   path) - and for pasted inspection/fix ledgers verifies each claim's
   DIRECTION, not just its existence. Check which end of a deque is
   newest: push_front means index 0 is newest, so .iter().rev() reaches
   the OLDEST entries - one inspection's 'cosmetic ordering' fix would
   have misreported old entries as new. Grep the test suite for tests
   that LOCK current behavior before dispatching any behavioral
   'improvement': record-time adaptation and cap-eviction arithmetic were
   both asserted as intended by existing tests and had to be retracted.
   When a fix names a dependency, grep the workspace Cargo.toml and
   lockfile first - the dep is often already enabled with an in-repo
   usage pattern to copy (an lru-cache fix needed no new dependency; the
   crate was already default-enabled with precedent in the proxy).
   Children then verify/refute against a known state instead of
   burning turns rediscovering what the parent already saw.
2. **Validation pair (read-only).** Two children, each owning a DISJOINT
   slice of the issue's claims (e.g. one takes the Python/runtime side,
   the other the scripts/docs/Rust side). Contract in every task: read-only
    - no repo edits, no git operations; verdict per claim as valid /
      invalid / partially-valid with line-numbered evidence, root cause, and
      a concrete fix proposal naming the exact file+function it would touch;
      return structured JSON (an output_schema makes the reports comparable
      across the pair).
3. **Fix pair (write).** Dispatched per task in the same order, with file
   ownership lifted straight from the validation proposals, so the two
   fix children never collide.
4. **Sequence tasks one pair at a time when the user lists them "in
   order"** - do not batch pairs across tasks even when the concurrency
   cap allows; the user's ordering is a dependency, not a suggestion.
5. **"don't commit or release" is the standing gate for these runs:**
   state it verbatim in every child context alongside the auto-commit
   warning, and verify `git status` is clean of new commits after each
   wave.
6. **Fold tightly-coupled tasks into ONE pair instead of two.** When an
   issue and a PR (or two listed tasks) touch the SAME files and one
   claims to fix the other, run them as a single pair with the ownership
   split across the shared surface (one agent owns the code fix, the
   other owns the tests/docs). Two separate pairs would double-edit the
   same file sequentially and force the second pair to re-merge the
   first's work; the validation pair and the fix pair both fold the same
   way. State the fold explicitly in the planning archive so the task
   list stays visibly complete.
7. **Reverify wave (standing default): a final READ-ONLY verification pair
   after all fix waves land.** When the user asks to "reverify
   everything extensively", dispatch one pair with a code/state split:
   agent A re-runs the entire test ladder (plain + env-variant) against
   the final committed tree, structurally spot-checks every task's fix
   in the code, and re-checks the defensive items; agent B verifies git
   state (exact commit lineage, clean tree, HEAD matches tree, no stray
   worktrees/branches), the removals, the planning archive's verdicts
   against the code, and the credit trail. Both are read-only - no edits,
   no commits, no remote contact. A reverify child that dies on 429 with
   its suite results in the transcript still counts: parent re-runs only
   the undelivered items.
8. **"Research more" = additional read-only pairs, not a fix wave.**
   When the user declines the fix and asks to deepen the investigation
   ("no hotfix yet, research more", "find more issues like that"),
   dispatch further disjoint READ-ONLY pairs before any implementation:
   a history/audit pair (git-log the bug's introduction, grep released
   tags for when it shipped, enumerate the full bug class), a fix-design
   pair (candidate matrix x pinned-test constraints, regression-test
   spec, downstream coupling audit), an empirical battery pair (probe the
   built artifact with a broad payload battery and score verdicts), and a
   system audit pair (map the whole pipeline, spec the ideal behavior per
   type). Give each pair a shared report-naming scheme in the repo notes
   archive (e.g. `ISSUE-<N>-AUDIT.md`, `-FIXDESIGN.md`,
   `-PREVIEW-BATTERY.md`) so the synthesis reads as one set. The parent
   VERIFIES the children's factual claims against the tree (grep the
   file:line, run the repro, read the workflow's actual gates) before
   reporting them as findings - a research child's "stale config claim"
   or "workflow discrepancy" is a hypothesis until the parent confirms
   it on disk. Do not implement anything in this mode; the fix wave is
   a separate later dispatch after the user approves a scope.
9. **End-of-session cleanup scope.** When the user asks to clear session
   artifacts, delete ONLY session-created paths (scratch dirs and files,
   temp branches, worktrees, duplicate clones) - list them with mtimes
   first, and verify pre-existing content in a shared root (e.g. an
   unrelated directory sitting beside the scratch) stays untouched. An
   explicit user cleanup request overrides any standing "keep scratch as
   evidence" default. Remember that symlinked homes resolve scratch and
   worktree paths to the SAME physical directory as their un-symlinked
   equivalent - a missing directory under the doubled path is the
   symlink expansion, not a vanished tree.

Pitfall (reproduction masking): on macOS, `/tmp` is a symlink to
`/private/tmp` and ADDS a path level, so a "shallow path" / depth-index
bug may not reproduce when staged under `/tmp` - stage at a genuinely
shallow path (fewer than 4 parents, e.g. `/opt/<name>`), or compute the
parents length directly to confirm the claim before declaring it
reproduced or non-reproduced.

## Pitfalls

- **`delegate_task` tasks JSON can refuse to parse ("Expecting ',' delimiter" / "Expecting property name" / "Invalid \\escape") - move the brief to a file and dispatch a short pointer.** Long contexts with quotes/escapes/unicode plus an `output_schema` block repeatedly fail serialization as "received a string that could not be parsed as JSON", wasting dispatch attempts. The failure is content-shape driven, not content-dependent: the identical brief dispatched fine as a minimal single-line task (one-line context, one-line goal, no `output_schema`) after failing three times as a multi-paragraph task (embedded `\\n` escapes, long prose, `output_schema` present) - so the fix is to CHANGE THE SHAPE, not to re-serialize the same text. The reliable fixes, in order: (1) write the long brief into a repo file (gitignored scratch dir) and dispatch a SHORT context that says 'read <path> and follow it exactly' - works even when every other fix failed; (2) collapse multi-paragraph context/goal to single-line prose (no embedded newline escapes) and drop the `output_schema` (the child's instruction in the goal suffices) - proven shape, dispatched on first try. Single-line by itself is NOT the fix: a one-line context still fails when the goal stays long and `output_schema` remains present (confirmed: 3 fails with multi-paragraph+`output_schema`, 1 success with short no-schema payload, then 2 more fails when single-line context was paired with a long goal + `output_schema`). The load-bearing halves are the SHORT goal and the ABSENT `output_schema`; shorten the goal prose and drop the schema together; (3) avoid backtick-escaped and unicode-escaped phrases. After TWO failed attempts with the same shape, stop re-trying that shape and switch to the file-pointer strategy - do not burn a third dispatch on the same payload. A validated payload built via execute_code + json.dumps proves the JSON is fine - the failure is in the call transport, not the payload content.
- **Goal text must contain no literal `{...}` brace literals.** `delegate_task`
  validates every goal for unexpanded template markers and refuses the whole
  batch with "contains an unexpanded template marker - subagents cannot
  resolve placeholders" when it finds one (e.g. a URL written as
  `{base_url}/chat/completions` or a JSON shape `{"model": id}`). Phrase URLs,
  JSON bodies, and paths in words ("POST to the chat-completions URL (config
  base_url plus \"/chat/completions\")") - the validator fires on the braces
  themselves, so even a legitimately literal `{model_id}` in prose will block
  the dispatch until you rewrite it. The same rejection hits ANGLE-BRACKET
  placeholders: command recipes written as `gh run watch <RUN_ID>` or
  `gh release view <TAG>` block dispatch until you substitute the concrete
  value or rephrase in prose ("the numeric run ID from the listing").
  Substitute every placeholder before calling; the validator names the
  offending task and rejects the whole batch.
- Sibling subagents race on shared files even when told otherwise -
  write_file refuses with `stale_write_blocked` ("modified by sibling
  subagent ... never read it") when the file changed since this task's last
  read; re-read then retry, or use patch for small edits (lenient). The
  guard has a documented kill switch, `HERMES_DISABLE_FILE_STATE_GUARD=1`
  (process-wide, subagents included; set in the private environment file).
  Net-new files (e.g. recreating a staged-deletion path in the worktree)
  bypass the guard entirely.
- An external auto-commit may sweep session work into commits behind your
  back (author = the account's git identity, mid-session timestamps, working
  tree suddenly clean). It violates the never-commit rule without any child
  running git. Warn children in their task context: "do NOT run git commit; an
  external auto-commit may run - ignore it". When verifying, check `git log
--oneline -5` + `git status` first; if auto-commits appeared, report them to
  the user rather than silently `git reset`-ing (the content is legitimate
  session work; fighting the auto-committer is a losing loop).
- **The session-start workspace snapshot goes stale by planning time -
  re-verify the tree before writing briefs.** The auto-committer can sweep
  "modified" files into commits between the first git status and the wave
  plan, so a wave scoped from the prompt snapshot mis-owns files that are no
  longer working-tree changes (observed: "4 modified, 2 untracked" at
  session start was 1 dirty submodule by planning time - the workflow edits
  and a new script had been committed). Before naming owned files in briefs,
  re-run `git status` + `git log --oneline -5` and diff the LIVE tree; the
  planning baseline is the tree at brief-writing time, never the snapshot.
- **Assign ONE owner for shared helper names.** When two children each need
  the same small helper (env gate like `live_tests_enabled`, path builder,
  URL constructor), name a single owner in the task text and tell the other
  to import it - two siblings defining the same name collide at the crate
  root (`E0252` duplicate-import) and the merge fix (drop one re-export) is
  precisely the cross-module mismatch the parent must resolve by hand.
- **Mid-wave, prefer the contained variant of a fix over its cross-file
  form - defer API renames and type changes that span another agent's
  owned files.** When a proposed fix's clean form renames a function or
  changes a field type whose call sites live in a sibling's files (a
  summary-function rename whose call site is in another agent's file; a
  Vec to VecDeque field change rippling into another agent's tests),
  implement the behavior change inside the single owner's file instead
  (restructure the body to commit state after assembly; drain the first
  element instead of switching the ring type) and record the deferred
  rename/type change in the planning archive for a later single-owner pass.
  A mid-wave type change breaks the sibling's own test runs for reasons
  outside its file, and the sibling cannot see the change to adapt.
- **When a product pivot forces a shared-type change mid-parallel, steer the
  dependent agent with the EXACT shape delta.** Some changes cannot be
  deferred (a pivot drops JSON schemas for DSL contracts, renaming/removing
  common fields): the owning agent changes the type, and every sibling whose
  crate constructs it breaks. Steer each dependent agent with the concrete
  delta (fields added/removed, new types) so its final state matches.
  Expect transient workspace failures until both land - a removed workspace
  dep breaks sibling MANIFEST load (`failed to load manifest for workspace
member`) until each drops its reference, and a changed struct breaks
  sibling construction (E0560 'no field'). Treat both as mid-wave noise,
  not regressions; the only run that counts is post-wave.
- **A wave's green suite in the PRIMARY crate is not green in the WORKSPACE
    - post-wave verification must run EVERY consumer crate's TEST target and
      grep the changed symbol workspace-wide.** `cargo test -p <primary>` does
      not compile sibling crates and `cargo check` does not compile `#[cfg(test)]`
      code, so a deliberate field-type change (a Vec promoted to VecDeque) broke a
      `.last()` call in a sibling crate's TEST module - caught only by `cargo test
-p <sibling>`, never by the primary crate's suite or a check pass. A child's
      'all consumers compatible' claim is a hypothesis until the parent greps the
      changed symbol's method usages across ALL crates (`grep -rn '<field>\\.'
crates/*/src --include=*.rs`), classifies each call against the new type's
      API (VecDeque has `back()`/`front()`; `last()` and `remove(0)` are Vec-only),
      and runs every consumer crate's test target. The same completeness applies
      before a tag/release push: the publish workflow's test job is where a missed
      sibling test-compile surfaces, after the Build workflow already succeeded.
- **A dependency that 'is already in Cargo.toml' can still be feature-gated -
  core/shared modules must only use UNCONDITIONAL deps.** `optional = true`
  behind a feature means the crate compiles without it when a sibling crate
  builds the lib with `default-features = false`; a state-module fix that used
  the optional `lru` dep compiled under the primary crate's default-feature
  build but failed E0433 ('cannot find crate `lru`') at the sibling's compile.
  Before using an existing dep in code a sibling crate builds, check its
  manifest line for `optional = true`; if gated, move it to an unconditional
  dependency (small pure-Rust crates are fine) or gate the usage behind the
  same feature.
- **Wave edits are not formatter-canonical - run the repo formatter and
  commit the canonicalization before CI's fmt --check gate runs.**
  Agent-produced Rust lands locally-consistent but not rustfmt-canonical;
  the release/check pipeline's `cargo fmt --all -- --check` fails on the
  wave's files. After the wave passes tests+clippy, run the formatter
  (`cargo fmt --all`), re-verify the suite, and commit the canonicalization
  as part of the wave - never tag/release with unformatted wave edits.
- **Review turns whose items are observations without numbered fixes are
  a planning-archive deferral, not a work dispatch** - log them in the
  planning archive with a deferred section, do not build briefs for
  them; only ledger items get agents.
- **Verify a brief's target paths against the repo before dispatch.** A
  child executes a confident brief literally: if the brief names a
  placement (an embed dir, a canonical home, a config location) that the
  parent never verified exists, the child creates it there - and the user
  catches the misplaced artifact after the wave. Before telling a child
  "create X at Y" or "use the canonical Z", grep the repo for the existing
  location (embed/source dirs, layout schema, setup.rs references, docs)
  and name the REAL path in the brief; an invented path in the brief
  becomes an invented directory on disk that a later wave must delete.
- **Scoping removals for a wave: inventory the target repo's current state
  first, and verify anything slated for deletion is not load-bearing in the
  degraded path.** When the task is "adapt the reference repo's latest
  changes here, remove unnecessaries", the removal half is real work:
  scripts referencing missing files, stale prepare hooks pointing at
  deliberately-removed machinery (e.g. package.json `prepare` -> deleted
  .githooks) get deleted as part of the adaptation. And a docstring's
  "remove X when Y lands" intent can conflict with X being the offline
  fallback: when the binary-first path already exists, the static mirror
  becomes GENERATED fallback data (regenerated from the live source), not
  dead code to delete - check what breaks when the binary is absent before
  scoping X out of a wave.
- **Mid-wave user corrections invalidate running briefs - steer every
  child whose scope they touch.** When the user changes scope mid-run
  (deletes installers, removes a subsystem, relocates a canonical path),
  dispatch the steer to EVERY running child whose brief names the now-gone
  paths or owns the affected files - not just the one you think is
  nearest. A brief that said "update the wrapper's install script" or
  "create directives/ under X" stays live in a child that already read it,
  and a child that finishes before the delivery boundary reports the steer
  as `missed_steer` in its completion - check for that marker before
  trusting the correction landed. The user's out-of-band message is
  authoritative mid-run steering, same as a steer call.
- **Mid-wave ownership transfer: steer the running child to DROP files a new
  task needs.** When a second task is dispatched after a wave has started and
  its file scope overlaps a running child's (e.g. a formatting fix arrives
  while a child is mid-write on the same docs), do not let both write -
  steer the running child to skip the overlapping files and leave them to
  the new task, and give the new task exclusive ownership of them. Steer
  text lands on the child's NEXT tool result and does not cut the current
  call; if the child finishes before a delivery boundary the steer is
  reported back as `missed_steer`, so check the completion report for that
  marker before trusting the transfer happened.
- **Mid-wave SAFETY HOLD: steer every running child whose brief mutates the held surface to PLAN-ONLY mode.** When the user signals a hold mid-run ("Current is actively being downloaded, be careful", a branch is live/being consumed), children already dispatched with git rm / file-edit briefs on that surface will happily mutate it before the steer lands. Steer ALL of them: no git rm, no file edits, no branch checkouts that alter the tree, no force ops - produce the plan document + deferred action list only, read-only inventory allowed. Then verify NO damage occurred before the steers landed (check branch + working-tree state of every held path), and steer any child that already switched branches to restore the prior state (e.g. switch the submodule back to Development) after it finishes plan-only work. Re-check with `git status`/`branch --show-current` per held repo before trusting the hold.
- **User pauses the whole operation mid-wave ("pause for now, continue tomorrow"): stop the running children, mark the planning archive PAUSED with an exact resume checklist, and fold late-arriving completions into the resume state instead of acting on them.** Distinct from the SAFETY HOLD above (which steers children to plan-only on a held SURFACE), a full-operation pause means: stop every live child (`delegate_task action=stop`), then write a `## PAUSED <date>` section into the plan file that lists (a) what landed and was verified, (b) the exact next dispatch (re-dispatch any stopped child whose verdict/deliverable file is missing - a stopped audit may have produced nothing), and (c) the orchestrator's remaining pass. Completion messages that arrive AFTER the pause are folded into that resume state ("B landed, verdict file verified; A interrupted - re-dispatch") - do not resume work on them. Keep the pause honest: stop children, record state, stop.
- **Remote-only inspection brief: convert every local read/run to its GitHub equivalent.** When the user dispatches a separate model with NO local access ("he'll only be working with the remotes"), the brief must not name local paths or `cargo test` runs. Give it: raw.githubusercontent URLs (branch + HEAD SHA from the commits API for stable blob refs), the contents API fallback for private repos (token note), Actions API runs (`/actions/runs?per_page=5` + jobs) instead of local test/clippy/fmt execution, blob-URL `#L<line>` evidence format instead of path:line, and a report-only contract (verdict table + findings + prioritized fixes for the parent to operate on). Keep the local version of the brief alongside for when the agent does get access.
- **Two pairs needing the SAME file for different concerns = sequential
  waves, not a split.** When pair A and pair B both must edit one shared
  integration file (`__init__.py` shim, a manifest) for different purposes,
  do NOT try to split it between them - queue pair B and dispatch it only
  after pair A's completion report lands, telling B the file is now free
  and to re-read the on-disk state (the first wave's edits are committed or
  uncommitted by the time it lands; do not assume the line numbers you
  memorized). Siblings racing the same file corrupt it even with disjoint
  intent.
- **New-files-only pairs can run parallel to file-owning pairs.** A task
  scoped to CREATE ONLY new files (a self-contained module + its tests +
  a design note) shares no write targets with a pair editing existing
  files, so dispatch it in the same batch - but state the new-files-only
  constraint verbatim in the brief, and keep it out of every existing
  file's ownership list.
- **Child-owned background process notifications reach the PARENT
  conversation.** A bounded `terminal(background=true, notify=true)` started
  by a child delivers its completion to the parent ("Background process
  proc_... completed ... Started by subagent sa-...") even when the child
  exits without reading its own result - the batch-complete message carries
  the output tail with the `missed_steer`/unread-result note. Treat those
  notifications as verification evidence: exit code + output tail in the
  parent's own conversation is a real gate result, usable even if the child
  died before composing its summary.
- **Do not run the workspace test while children still hold the tree open.**
  A half-written sibling file breaks the build transiently mid-wave; a child's
  mid-run `cargo test` can fail for reasons outside its own file. The only
  run that counts is the post-wave verification after all children have
  exited - treat any mid-wave failure as noise.
- **Use absolute paths for the parent's own file edits after a terminal
  `cd`.** The session cwd persists across calls; once a child-run `cd`
  (or your own) moves it, `patch`/`write_file` relative paths resolve
  against the NEW cwd and fail with "Failed to read file: <wrong path>".
  Edit targets as absolute paths whenever the working directory has drifted.
- Children burn time re-iterating on the same compile/test error; steer after
  ~5 minutes on the same error class with "your file is integrated and green -
  stop and report" rather than letting them loop.
- **Children will reuse stale /tmp harnesses from earlier probes - forbid it
  in the brief and name the real source.** A child tasked with re-running a
  battery found the session's OLD raw-ctypes probe scripts in /tmp and
  started adapting them instead of driving the actual plugin source - the
  exact code class that SIGSEGVs (unset restype truncates the 64-bit
  pointer). Briefs for probe/battery tasks must state: use ONLY the real
  source path (import the plugin package, drive its own loader functions),
  NEVER hand-roll raw ctypes probes, and keep all scratch in the repo's
  gitignored temp dir - never /tmp. The user caught this by asking "why is
  the agent testing with code inside there instead of here?" - verify the
  child's transcript shows it reading the REAL source before trusting its
  methodology claim.
- A failed `git mv` on a gitignored directory creates an empty destination
  dir, so the follow-up `mv` nests instead of replaces - check for a pre-existing
  destination before moving.
- **Multi-agent doc restyle = one pair, exact skeleton, byte-identity
  diff.** When several earlier subagents each wrote a file in the same
  class (classification passes, per-area docs) and the user asks to
  'improve the markdown', dispatch ONE restyle pair with disjoint
  ownership (2 files each) and a VERBATIM unified spec: exact H1 title
  format, exact numbered section skeleton (1..N with the same headings),
  one cross-reference line under H1, and a content-preservation rule
  ('tables/codes/diagrams byte-identical; move existing sections into the
  skeleton'). Tell each child explicitly which pre-existing sections MOVE
  where (amendment block under section 3 -> section 4, count under section
  4 -> section 5) - children do not infer renumbering. Verify after the
  wave: `npx prettier --check` on the whole set, line-count delta within
  ~5% (additions-only), and a byte-identity diff of table rows against
  saved originals. An index README + the taxonomy doc itself are the
  orchestrator's own edits, done while the pair works.
- **Doc-rewrite briefs must disambiguate DELIVERABLE location from SCRATCH
  location - the user reads the scratch rule as a deliverable restriction.**
  In a full docs rewrite ("completely rewrite ./docs"), the user pushed
  back on briefs that said "Scratch goes in .hermes/tmp": he read that as
  "agents may not write outside .hermes/tmp" and asked why they weren't
  rewriting ./docs. Every brief must lead with the OWNED FILES section
  naming the real ./docs paths (that is where deliverables land), then
  state the scratch rule as scratch-only (probe files, intermediate notes,
  and the brief files themselves live in the gitignored temp dir so the
  auto-committer never sweeps them). Also name the orchestrator-owned
  index READMEs explicitly ("Do NOT edit docs/README.md / root README.md -
  orchestrator rewrites them LAST") so the user sees the final goal is the
  index rewrite, not a permanent restriction.
- **Every static value in an instruction file (brief, skill, plan file)
  must carry a property - a derivation path - so it is inferable at
  read/write time; a bare constant is uninferable the moment the tree
  moves.** The rule, stated generally: values "must have properties
  not just static values, that are un-inferable at read write time." This
  generalizes the branch-link rule (which branch a file lives on) to every
  number and name an instruction states: versions ("1.4.6") -> "read
  BINARY_VERSION at handshake time, never a remembered constant"; ports
  (":9798") -> "the token listener port is a config property, read it live
  from the running proxy / aphrodite.toml ports"; counts (26 content
  types, 6 hooks, 13 tools) -> "count from source, never inherit a stale
  count"; thresholds (512B) -> "read the live [compression] value, this row
  is a shipped-default snapshot"; tag examples ("Aphrodite/v1.4.3-rc.1")
  -> "pattern vX.Y.Z-rc.N, actual version read from BINARY_VERSION at tag
  time". Config snippets presented as working must have a CONSUMER check:
  auto_expand / auto_expand_limit are parsed fields (config.rs:250-251)
  echoed only in a status response (proxy.rs:2668-2669) with no active
  compression consumer - a skill example that presents them as a toggle
  contradicts the live source and misleads the next session; annotate the
  example with the file:line proving current behavior. ALGORITHM and
  MODULE-LOCATION claims are the same class of static: a skill's pattern
  section can name a hash algorithm or a store the code moved away from
  (content addressing is BLAKE3 40-hex via `blake3::hash`, while SHA256
  survives only as the download checksum verifier; a Python
  `_inline_store` cache died with the pure-loader rewrite and the store
  now lives in the Rust engine's `inline_store`). Grep the source before
  repeating any algorithm/store claim, correct it IN PLACE with the real
  name + source anchor and mark the old shape historical - never leave
  the stale claim with an 'UPDATE: actually...' note appended. Search the
  whole instruction surface (skills, briefs, plan files) for the same
  class, not just the one value the user flagged.
- **Mermaid flowchart labels with unquoted braces break GitHub rendering
  with a DIAMOND_START parse error - quote every label containing special
  characters.** A doc rewrite of a repo whose diagrams render on GitHub
  ships broken diagrams when labels like `B[HintContext: {Code(rust)}]`
  are left unquoted: mermaid reads the `{` as a diamond-shape start.
  Fix: `B["HintContext: {Code(rust)}"]`. Also avoid unicode symbols in
  labels (use `x4` not the times symbol - non-ASCII can trip the parser
  or render oddly). State this rule verbatim in every doc-rewrite brief
  ("every mermaid fence must render on GitHub; quote labels with special
  characters using the double-quote form") and after the wave run a scan
  pass for `X[label-with-{}-or-special-chars]` patterns across ALL mermaid
  fences in the tree, not just the files each child touched. The scan is
  `scripts/mermaid-render-lint.py` in this skill (read-only; enumerates
  real line numbers - never m.start()+i, which is a character offset that
  reports nonsense lines for long files).
- **AGENTS.md / CLAUDE.md are guard-protected agent-instruction files: a patch triggers a per-edit approval prompt; a timed-out prompt blocks ONLY that edit ("Silence is not consent"), while sibling patches on the same file in the same batch can still land.** When a wave ends with parent-side map edits (skill lists, version lines, doctrine fixes in AGENTS.md), send them as separate small patches and expect some to be blocked when no user is present; never retry a blocked edit via another path (terminal, write_file, a reworded patch) - the block is binding. Verify which patches landed with a grep and leave the blocked line for the user. Full recipe for rewriting a repo skill library with parallel waves: the skill-library-refactoring skill.
- **Verify-against-source doc rewrite = planning-archive contract + per-child
  discrepancy logs + orchestrator-owned link pass.** When the user asks to
  "completely rewrite the docs, they are stale" (as opposed to a restyle of
  already-correct files), dispatch pairs with disjoint file ownership per
  doc cluster, but FIRST write a plan file in the repo planning
  archive (e.g. `.hermes/notes/ops/DOC-REWRITE-<date>.md`) that carries the
  whole contract: verified ground facts (versions, hook/tool counts, default
  branch - grep them from source BEFORE dispatch, never trust the old docs),
  the target structure table, the wave plan with exact per-child ownership,
  the per-child contract, and an append-only discrepancy-log section.
  Every child contract then says: read the existing doc, grep-verify EVERY
  claim against live source (still true -> keep and rewrite clean; false/stale
  -> correct AND append to its OWN discrepancy file
  `DISCREPANCY-<PAIR>-<child>-<date>.md` - never a shared log, siblings
  appending one file race); public prose carries no file:line citations;
  no internal process artifacts; repo links carry the branch the FILE
  lives on - infer per file at edit time, never blanket-assume:
  `git symbolic-ref --short HEAD` tells which branch the file was halted
  on and will be viewed from, and THAT branch goes in the link (e.g. on
  the Development working line every .md link says `tree/Development`
  even when the GitHub default branch is `Current` - the default branch
  is the distribution line, not the link target; the stale-docs tell is
  a link whose branch doesn't match the file's actual branch, and a
  hardcoded-branch instruction in a brief is the same class of error).
  The orchestrator owns the index README(s) and the
  GLOBAL link pass, done ONLY after every agent finishes - children leave
  internal cross-file links to moved/renamed targets alone and never touch
  the index files, because renames during the wave invalidate any link pass
  run earlier. Also: borrow verified private-archive content (e.g. adapt
  `.hermes/uml/` flow traces into public architecture docs) instead of
  rewriting from scratch - the private archive is often newer than the
  public docs.
  The final pass is NOT just docs/README.md: it must also sweep the ROOT
  README for branch links that don't match the file's branch context (a
  link must say the branch the file is viewed from - on the Development
  line that is `tree/Development`, not the GitHub default branch) and
  for links to files the audit archived (a root README linking
  `docs/APHRODITE-HEADROOM.md` goes dangling the moment the audit moves
  that file to the internal notes archive - re-point the link to a
  surviving doc or drop it, and verify the license badge points at the
  right branch too). Run the whole battery before declaring done:
  wrong-branch link count == 0, dead relative links == 0 (resolve every
  `[x](path)` against its file's dir; skip http/mailto),
  `scripts/mermaid-render-lint.py` risks == 0 across ALL fences,
  prettier --check on every touched file - then mark the plan file
  COMPLETE. `scripts/doc-link-scan.py` in this skill runs the link half
  of that battery (wrong-branch grep + dead-relative-link resolver).
  While the final pass runs, flag (don't investigate or revert) any
  working-tree changes OUTSIDE the owned file set - another session or
  the user's tooling may be editing the tree in parallel; leave them.
- **A child's "no stale phrasing remains" verification is scope-limited -
  re-sweep yourself with patterns the child was never told to check, over
  files it did not own.** A doc/classification-refresh child reported a clean
  grep pass and still left: version constants it was not briefed on (binary
  1.5.0 rows when the live pointer said 1.5.1; a plugin 2.2.0 row when `git
tag` ended at v2.1.5), rows naming deleted files (a hotreload test that no
  longer exists in the tree), and a stale diagram node in a file outside its
  ownership list (a mermaid `hotreload/` node in the component doc). The
  child greps for the exact phrasing it was told to fix; it does not grep
  for version numbers, deleted-file names, or unowned files. After any
  delegated refresh, run your own sweep: (1) grep the WHOLE tree (not just
  owned files) for old version constants and deleted-file names; (2) verify
  every version constant against live ground truth (`git tag | sort -V |
tail`, `grep '^version' <manifest>`); (3) check diagram/fence files the
  child never touched for stale nodes. Fix the leftovers yourself in-session
    - the sweep is part of "finish the sweep directly", not another dispatch.
- **Long manual conversion/doc passes run at the user's pace: small
  edits over time, one unit at a time, stop cleanly at budget.** When the
  user says "small edits over time, take your time" (or the brief is a
  large manual conversion - prose-to-tables, restyling, per-file
  restructure), the child must: work incrementally (ONE table/file/edit
  per step), re-read the file before every edit, verify each unit's
  integrity as it goes (a table's rows after each conversion), never
  rewrite a whole file in one pass, and if the budget runs out STOP with
  a continuation list (files still needing the pass) rather than rushing
  the tail. The parent then re-dispatches a resume scoped to the listed
  remainder. Rushing produces the exact breakage the slow pace exists to
  prevent (broken tables, lost content).
- **Merge/conflict-resolution children: manual per-file passes only - no bulk, no scripts, no index shortcuts.** For squash-merge conflict resolution and staged-review passes, briefs must state: one file at a time (read -> decide -> edit -> `git add` individually); NO loops, NO bulk commands, NO python; NO `git restore` / `git reset` / `git checkout HEAD --`; removals via plain `rm`, content fixes via patch/write_file; the staged index stays untouched except the per-file add. The user rejects bulk passes outright ("no bulk passing", "no BULKING OR PYTHON SCRIPTS") and wants the staged proposal preserved while corrections accumulate as unstaged changes.
- **Subagent edits on tab-indented files silently lose tabs - byte-compare against the git stage.** patch/write_file strip leading tabs and trailing newlines; when a child must reproduce a file byte-identically (keep-HEAD conflict resolution, formatter-canonical content, restoring a staged-deletion path from `git show HEAD:<path>`), verify each result against the stage blob (`git show :2:<path>` or an empty `git diff --cached`) and repair drift before moving on - a staged diff where none should exist is the tell. write_file cannot express a final newline, so after a blob restore append it with `printf '\n' >> <file>` and re-`cmp` against the blob.
- **A doc-rewrite child can smash an entire table onto ONE physical line with literal `\n` escapes - verify row structure after the wave and restore from HEAD, not the index.** When a child edits a one-row-per-line markdown table (HPC classification passes, per-area docs), a bad write emits literal `\n` escape sequences instead of real line breaks: dozens of rows collapse onto a single line (15KB+), which is what a user sees as "truncated on the same line". The signature: `grep -c '\n  |' <file>` > 0, or a max line length in the 5-digit range (`awk '{print NR": "length($0)}' <file> | sort -t: -k2 -rn | head`). The corruption is usually STAGED (the auto-committer swept it into the index) - `git checkout -- <file>` alone does NOT fix it because checkout restores from the INDEX, which still holds the corruption; unstage first (`git reset HEAD -- <file>`) then checkout. Before restoring, `git log --oneline -- <file>`: the child's intended fixes may already be committed separately (auto-commit), making a HEAD restore lossless - then re-apply any missed intended edits yourself. After the restore, verify escapes == 0, max line length back to normal, and prettier passes. The child's completion summary can itself arrive truncated mid-sentence - it is evidence of neither file health nor corruption; the on-disk row structure is the only truth.
- **"We'll commit as they go" is the standing commit cadence on
  long repo refactors**: leave changes staged/unstaged for the user's
  Save tool or the external auto-committer to sweep - never commit
  yourself, and expect the remote tip to advance between waves. After a
  wave lands, confirm with `git log <remote-branch>` not `git status`;
  `git push` may report "Everything up-to-date" because the
  auto-committer already pushed. This extends to SPEC AUTHORING: never
  write commit+push steps into a child spec - the default deliverable is
  uncommitted working-tree changes. A spec that does carry a Git section
  gets overridden mid-flight ("don't make them commit, we'll do that on
  the side") and you must steer EVERY running child whose spec names a
  commit step, then check each completion for `missed_steer` before
  trusting the override landed.
- **Provider per-minute inference quota caps parallel fan-out: 429s mean
  the wave is too wide - re-dispatch failed units SEQUENTIALLY.** On
  endpoints with a per-minute request quota, a 4-way parallel batch can
  kill 2-3 children with HTTP 429 "rate limiting: inference request per
  min rate reached" within ~50s, while 1-2 wide batches complete fine.
  The runtime cap is not the binding constraint; the provider's per-minute
  quota is. When 429s appear in a batch: let the surviving children finish
  (their completions still arrive), then re-dispatch the failed units one
  at a time or in a 2-wide wave - never re-fan-out the ORIGINAL width
  (a 7-wide fan-out killed every child in one wave; 2-wide re-dispatches
  landed 4/4 across two waves). Each small retry gets the full quota window
  and the retry pattern has been reliable across sessions. A child that died on 429 with its partial results in the
  transcript still counts - re-dispatch only the undelivered items.
  **Two death signatures in a quota-killed wave - verify the tree before
  re-dispatching, the signature decides the retry.** An INSTANT death
  (sub-second, died at or near its first model call with 429 or 401) has
  produced NOTHING - check the tree for absence (the expected new
  dir/files never appeared) and re-dispatch, no lost work. A LATE death
  (long runtime, died composing the final summary) has USUALLY landed the
  full deliverable - its partial output carries the completed results JSON
  (files-rewritten list + counts + gates), the batch reports it "failed",
  and the on-disk evidence confirms it (modified files + the discrepancy
  log it was told to create). Do NOT re-dispatch a late-death child; the
  tree already holds the deliverable and a re-run burns a full quota
  window. In one wave this session: 3 children died instantly (empty
  tree, re-dispatched one at a time) and 1 died late after writing all 5
  files + its discrepancy log (verified on disk, not re-dispatched).
  **An instant 401 (auth) death is a TRANSIENT blip, not a config
  problem - solo re-dispatch succeeds; do not wait for the batch.**
  On the Cloudflare Workers AI provider, children die sub-second with
  HTTP 401 "Authentication error" at their first model call while the
  parent session keeps working - the same transient pattern as 429.
  Proven 4/4 in one session: every instant-401 death followed by a solo
  re-dispatch that landed (417-956s runs). Re-dispatch the dead task the
  moment the per-task early-warning arrives - even while a sibling still
  runs (2-wide is safe) - never wait for the batch-complete message, and
  never report it to the user as hard-broken provider auth.
  **After a MIXED 429+401 cluster, probe with ONE minimal agent before
  re-fanning out - the failure window can outlast the batch and swallow a
  smaller retry too.** A 3-wide re-dispatch minutes after a 7-wide 429 kill
  still died at every child's first call with 401s; a single minimal probe
  dispatched minutes later landed clean. Recovery sequence that held: let
  the batch fully settle (partial results in transcripts still count),
  dispatch one small probe task, and re-fan out 2-wide only after the probe
  completes. The same 3-wide width passed earlier in the same session, so
  width alone is not the deciding factor - the provider's current window
  state is; the probe tells you when it has recovered.
- **"Completed" status with a mid-design transcript means the deliverable
  NEVER landed - verify the artifact, not the status.** A child that reports
  completed in a SHORT duration (hundreds of seconds, not the full window)
  whose transcript tail is still design-phase thinking ("Let me now design
  the implementation") may have produced nothing: the runtime marks it done
  after its last call and the summary is whatever it last thought. The
  failure is silent - no timeout, no schema-death, no error. Before trusting
  any completion, check the ON-DISK evidence: `git log --oneline -5` for the
  expected commit, grep the expected region/artifact, run the new test.
  This session: a WS3 + residual-#3 agent "completed" in 489s having only
  written its design note - the parallel preview arms were still in
  proxy.rs, no commit existed, and the work was silently absent until the
  parent grepped the tree. Re-dispatch such a child with the hard
  requirement spelled out: "the code change AND its tests must exist and
  pass before you report done".
- **Standing rules for subagent deliverables - state them verbatim in
  every brief:** (1) deliverables go ONLY into the repo's notes taxonomy
  (e.g. `.hermes/notes/<category>/`), never into any scratch dir - a
  stray project dir was removed twice and the agent that recreates it is
  killed; the only scratch is the repo's gitignored temp dir and the
  session scratch tree. (2) External documents are ADAPTED, never copied -
  when a brief says "rewrite X from Downloads into notes", the child must
  distill the source into ORIGINAL prose in the repo's voice, re-deriving
  any sketches; pasting the source's sections or code blocks verbatim is a
  violation ("I said rewrite, not copy over"). (3) Zero local absolute
  paths in any deliverable - scrub home- or volume-style absolute paths
  to generic phrasing before writing.
- **An exported env var is a SUITE-WIDE landmine - config tests must be
  hermetic.** A probe that exported `APHRODITE_PREVIEW_MAX_CHARS` into the
  parent shell silently broke 21-25 `cargo test` config-precedence tests
  per run with VARYING failures: the resolution chain is override -> env ->
  toml -> default, so the env value beat the TOML, the first assert failed,
  the test panicked BEFORE its restore, and the leaked process-global cap
  poisoned every later preview test. The suite failure looked like a race
  (varying counts) but reproduced single-threaded - the tell is truncation
  at a value matching an exported knob. Config/precedence tests must
  remove_var the env var for their duration and restore it after; process-
  global atomics mutated by tests need a shared module mutex guard taken by
  every content-asserting test. A child's clean env does NOT prove the
  parent's env is clean - re-run the suite in your own shell after any
  env-var change.
- **A child that times out at the hard cap (1800s) with work in the
  transcript has USUALLY finished the deliverables and died composing the
  final summary** - long build/test-ladder children hit this after their
  last real tool call. Treat timeout like schema-death: read the transcript
  tail for the last completed verification ("[bench_05] OK RUN_EXIT:0",
  "22 passed"), then verify the on-disk state (version files, new files,
  test run) yourself. Do NOT re-dispatch - the tree already holds the
  deliverable and a re-run burns another full window. Only re-dispatch when
  the transcript shows the work genuinely unfinished mid-stream.
- **The batch timeout always eats the LAST scope item - schedule
  deterministic final steps (config restore, cleanup, matrix
  reconciliation) as PARENT work, never as the final step of a batch.**
  Three consecutive 30-min capture batches each timed out with the
  mandatory runtime-config restore unexecuted (the tail is where the
  timeout lands). The restore is deterministic (known shipped values) and
  the parent did it surgically: patch each capture line back to the
  default, verify `grep -c '<capture-marker>' == 0` + config parses. If a
  final step must be in a batch, put it FIRST or in its own tiny dedicated
  batch - never last.
- **A timeout with PARTIAL progress is a resume, not a re-run.** Distinguish
  "complete-in-transcript" timeouts (deliverable done, no re-dispatch) from
  "mid-scope" timeouts on multi-batch missions (16 of 18 variants captured,
  scope far from done): the artifacts on disk + the planning/exchange file
  survive the child; re-dispatch a RESUME batch scoped to exactly what
  remains, reading the surviving progress file.
- **Multi-batch capture/example missions: persist progress in a shared
  exchange file so any child timeout loses only its own batch.** A 100+
  variant capture mission cannot fit one 30-min child. Structure it as
  batch-scoped dispatches (P0/P1/P2/...), a shared notes file with
  REQUEST <id> / DONE <id> entries (producer child appends DONE, cataloger
  child appends REQUEST), and the coverage matrix as the index. Each batch
  starts by reading the notes file and continues the queue; a timeout loses
  at most the running batch. Make each capture session MULTI-VARIANT (one
  trigger prompt producing several response types - ls + diff + read + build
  in one session) so a batch covers several variants per session instead of
  one; one-variant-per-session makes the mission outlast every budget.
- **A release/ceremony track runs in a LINKED WORKTREE, not the polluted
  main checkout.** `git worktree add <path> <branch>` (never `cp -rf` a
  repo - it duplicates the object store and aborts), `submodule update
--init --recursive` inside, and remember branch exclusivity: a branch is
  checked out in exactly one worktree, so the linked worktree takes the
  release branch while the main keeps the review branch; cross-branch reads
  and merges work from either. Clean up with `git worktree remove` +
  `prune` when the track closes.
- **A scratch clone diverges from the canonical checkout mid-session - diff
  BOTH directions before porting.** When the user's tooling auto-commits
  session work into the canonical location (a submodule linked to the
  remote) while children edit a sibling scratch clone, the two trees carry
  different commit sets by the time the work lands. Before copying whole
  files from scratch to canonical, diff both directions: a full-file copy
  is only safe when the target side is a strict subset of the source (any
  target-only lines mean the copy would wipe them). Verify the full test
  ladder in the TARGET location after the port, move any gitignored
  planning archive there too, then remove the scratch clone, its git
  worktrees, and its temp branches. Deleting the scratch clone before the
  port is finished destroys uncommitted work - port first, delete last.
    - Fix waves that touch platform-gated ctypes code (e.g. a win32 branch)
      must verify that branch's logic on the host OS with a fake
      `ctypes.windll`. Naive fakes (class methods, instance methods) silently
      produce all-True results; the working recipe is closure-based plain
      functions in a SimpleNamespace - see
      `references/verify-platform-gated-ctypes.md`.

## Wave gates

| Gate        | Check                                                     | Pass condition                                    | Failure response                                 |
| ----------- | --------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------ |
| Dispatch    | File ownership disjoint across all children in the batch  | No shared write target                            | Re-split ownership before dispatching            |
| Completion  | On-disk evidence matches the child's report               | grep/test proves the deliverable landed           | Re-dispatch only the undelivered items           |
| Integration | Workspace build + tests in the PARENT env after each wave | `cargo test` green; cross-module mismatches fixed | Fix mismatches yourself, never via another child |
| Final       | Full ladder + scans (fmt/clippy, link, mermaid, prettier) | All scans clean; plan file marked COMPLETE        | Fix in-session; record results in the archive    |
