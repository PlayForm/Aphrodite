---
name: plan
description: "Use when asked to plan, not execute. Save to .hermes/plans/. Absorbed the old plan-writing skill: plan-mode turns produce a markdown plan, never code."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
category: engineering
category_taxonomy: engineering/plan
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, planning, sizing, handoffing, delegating]
        related_skills: [test-driven-development, parallel-delegation-execution]
status: active
---

# Plan Mode

Plan mode means this turn produces a markdown plan, not code. Inspect
freely, write nothing except the plan file, and leave the workspace exactly
as you found it.

Write implementation plans for an implementer with zero codebase context and
questionable taste. Document everything they need: which files to touch,
complete code, test commands, docs to check, how to verify. Give them
bite-sized tasks and commit frequently. Assume the implementer is a skilled
developer who knows almost nothing about the toolset, problem domain, or
good test design.

**Core principle: a good plan makes implementation obvious. If someone has
to guess, the plan is incomplete.**

## When to Use

- The user asks for a plan instead of execution ("plan this", "/plan",
  "don't code yet").
- The user wants a written proposal to review before any work starts.
- A task is underspecified and the plan itself is the deliverable.
- Implementing multi-step features (e.g. a new CCR engine path in
  `crates/aphrodite-hermes`)
- Breaking down complex requirements
- Delegating to subagents via `delegate_task`

Don't skip when:

- The feature seems simple - assumptions cause bugs
- You plan to implement it yourself - future you needs the guidance
- Working alone - documentation matters

## Prerequisites

- An active workspace (local, docker, ssh, modal, or daytona backend).
- Read access to the repo or conversation context you are planning against.
- Understanding of the feature requirements, constraints, and acceptance
  criteria.
- The project's test command (e.g. `cargo test -p aphrodite-hermes`;
  canonical gates in `.hermes/AGENTS.md`).

## How to Run

1. Inspect the repo or conversation context with read-only tools
   (`read_file`, `search_files`, read-only `terminal` commands).
2. Write the plan with `write_file` to
   `.hermes/plans/YYYY-MM-DD_HHMMSS-<slug>.md` relative to the active working
   directory - Hermes file tools are backend-aware, so the relative path
   keeps the plan with the workspace on every backend.
3. If the runtime provides a specific target path, use that exact path.
4. Reply briefly: what you planned and the saved path. Then offer execution
   via `delegate_task`.

## Quick Reference - Plan Structure

```
# [Feature] Implementation Plan

> **For Hermes:** Implement this plan task-by-task with `delegate_task` (one subagent per task).

**Goal:** [one sentence]
**Architecture:** [2-3 sentences]
**Tech Stack:** [key technologies/libraries]
---
```

Each task:

```
### Task N: [Descriptive Name]

**Objective:** [one sentence]

**Files:**
- Create: `exact/path/to/new_file.rs`
- Modify: `exact/path/to/existing.rs:45-67` (line numbers if known)
- Test: `crates/aphrodite-hermes/tests/path/to/test_file.rs`

**Step 1: Write failing test** → code
**Step 2: Run test to verify failure** → `cargo test -p <crate> <test_name> -- --exact`, Expected: FAIL
**Step 3: Write minimal implementation** → code
**Step 4: Run test to verify pass** → Expected: PASS
**Step 5: Leave the change uncommitted** → repo convention: the auto-committer/sweeper handles commits
```

## Quick Reference - Plan Mode Rules

| Allowed                                                                  | Forbidden                                                  |
| ------------------------------------------------------------------------ | ---------------------------------------------------------- |
| Read-only inspection (`read_file`, `search_files`, read-only `terminal`) | Editing project files other than the plan                  |
| Writing the plan markdown                                                | Mutating terminal commands, commit, push, external actions |
| Asking a brief clarifying question                                       | Implementing code                                          |

## Procedure

### 1. Understand requirements

Read the feature requirements, design documents, acceptance criteria, and
constraints. If the request is genuinely ambiguous, ask one brief
clarifying question instead of planning blind.

### 2. Explore the codebase

Use Hermes tools: `search_files("*.rs", target="files", path="crates/")`
for structure, `search_files` for similar patterns, `read_file` on key
files, and `search_files("*.py", target="files", path="plugins/aphrodite/")`
for existing plugin tests.

### 3. Design the approach

Decide the architecture pattern, file organization, dependencies, and
testing strategy. Include, when relevant: Goal, Current context /
assumptions, Proposed approach, Step-by-step plan, Files likely to change
(exact paths for code-related tasks), Tests / validation (likely test
targets and verification steps), Risks, tradeoffs, and open questions.

### 4. Size the tasks

Each task = **2-5 minutes of focused work**, one action per step. Too big:

```markdown
### Task 1: Build the retrieval pipeline

[50 lines of code across 5 files]
```

Right size:

```markdown
### Task 1: Add the CCR entry type with hash field

### Task 2: Add size/type classification to the entry

### Task 3: Create the marker encode utility
```

Order tasks: setup/infrastructure → core functionality (TDD per task) →
edge cases → integration → cleanup/documentation.

### 5. Add complete details

- **Exact file paths** - `crates/aphrodite-hermes/src/engine/mod.rs`, not
  "the engine file"
- **Complete code examples** - copy-pasteable, not "add validation"
- **Exact commands with expected output** - `cargo test -p
aphrodite-hermes`, expected: `N passed`
- **Verification steps** that prove the task works
- For Aphrodite work, name the crate (`crates/aphrodite`,
  `crates/aphrodite-hermes`, `vendor/headroom`, `plugins/aphrodite`) and
  the exact gates from `.hermes/AGENTS.md` (`cargo test -p <crate>`,
  `cargo fmt --check`, `cargo clippy`, `ruff`,
  `npx prettier --check .hermes/**/*.md`) in the validation section.

### 6. Review the plan

Check: tasks sequential and logical; each bite-sized (2-5 min); file paths
exact; code examples complete; commands exact with expected output; no
missing context; DRY, YAGNI, TDD applied.

### 7. Save and hand off

Save the plan under `.hermes/plans/` (or the provided target path), then
offer: _"Plan complete and saved. Ready to execute with `delegate_task` -
a fresh subagent per task, spec-compliance review then code-quality
review. Shall I proceed?"_ When executing, dispatch a fresh `delegate_task`
per task with full context and two-stage review; proceed only when both
reviews approve. Implementers leave changes uncommitted for the repo's
sweeper (Aphrodite repo convention).

## Principles

- **DRY** - extract shared logic; don't copy-paste validation into 3 places
- **YAGNI** - implement only what's needed now; "flexibility for later" is
  scope creep
- **TDD** - every code-producing task runs the full cycle: write failing
  test → verify it fails → minimal code → verify pass (see
  `test-driven-development`)
- **Frequent small changes** - one task, one coherent change; in the
  Aphrodite repo, agents leave changes uncommitted for the sweeper (the
  plan's commit steps run at the parent or sweeper level)

## Pitfalls

- **Do not implement.** No code, no file edits beyond the plan, no mutating
  commands - a plan turn that changes the repo defeats the user's intent.
- **Do not guess an underspecified task.** If the request is genuinely
  ambiguous, ask one brief clarifying question instead of planning blind.
- **Do not invent a save path.** Use the runtime-provided target path when
  one exists; otherwise timestamp the filename yourself under `.hermes/plans/`.
- **Infer, don't interrogate.** If no explicit instruction accompanies
  `/plan`, infer the task from current conversation context.
- **Keep tasks bite-sized.** A "Build the retrieval pipeline" task with 50
  lines across 5 files is un-reviewable; split until each task is one
  action.
- **Write complete code, never stubs.** "Step 1: Add validation function"
  without the function is a guess - include the full copy-pasteable code.
- **Give exact commands with expected output.** "Test it works" is not
  verifiable; "Run `cargo test -p aphrodite-hermes`, expected: 3 passed"
  is.
- **Name exact file paths.** "Create the engine file" leaves the implementer
  guessing; `crates/aphrodite-hermes/src/engine/mod.rs` does not.
- **Never skip the RED step.** A test that passes on first run tests
  existing behavior - fix the test, not the verdict.
- **Don't bundle scope.** Each task changes only what its objective names;
  no "while I'm here" refactors.

## Verification

- [ ] Plan saved under `.hermes/plans/` (or the provided target path)
- [ ] No project files modified; no mutating commands run
- [ ] Plan is concrete: exact paths, steps, and verification for code tasks
- [ ] Header present: Goal, Architecture, Tech Stack
- [ ] Each task has Objective, Files, and Steps with commands + expected
      output
- [ ] Tasks are 2-5 minutes each, sequential and logical
- [ ] File paths exact; code complete; verification steps present
- [ ] DRY, YAGNI, TDD, frequent small changes applied
- [ ] Execution dispatch offered (`delegate_task` per task)
