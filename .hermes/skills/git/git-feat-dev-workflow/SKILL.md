---
name: git-feat-dev-workflow
description: "Use when running the feat-dev reverse-PR branch workflow to contribute upstream (e.g. hermes-agent) from the Aphrodite dev environment."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
category: git
category_taxonomy: git/git-feat-dev-workflow
date: 2026-09-25
metadata:
    hermes:
        tags: [git, workflow, branching, pull-request, aphrodite]
        related_skills: [git-operations]
status: active
---

# feat-dev Reverse-PR Workflow

A branching discipline for contributing to `hermes-agent` (or any repo where
you want to control _what_ leaves your tree, and where upstream does NOT use
your methodology, so Tier-2 branches use **regular PRs**). The Aphrodite dev
environment runs this when Hermes-integration work (hooks, plugins) needs to
reach upstream as clean PRs.

## When to Use

- Contributing to hermes-agent or a fork you maintain: new feature, fix, or
  cleanup that must reach upstream as a clean PR.
- After `hermes update` advanced `main` and your local feature branches silently
  diverged.
- Deciding how to pull upstream changes in without letting them force your tree.
- Working from the Aphrodite dev environment (Development working copy,
  remote `Source`): its external auto-committer sweeps working-tree changes,
  so record HEAD before cutting branches and verify with `git log -1` - never
  assume a clean `git status` means nothing moved.

## The imperative: location, location, location

**The branch a PR is launched FROM is the source of truth - not upstream's
`main`.** This is local-first: nothing here pushes, fetches, or mutates any
remote by default. `Fork` and `origin` are left exactly as found - _they_ pull
from your published branches; you do not chase their moving target. Record, for
every feature, **where it started** (which base commit / which branch it was cut
from): that provenance is what makes a clean PR possible and keeps old open PRs
isolated.

## Topology (3 tiers)

| Tier    | Branch               | Based on             | Purpose                                                    |
| ------- | -------------------- | -------------------- | ---------------------------------------------------------- |
| Trunk   | `feat-dev/trunk`     | `main` (NO upstream) | ALL your features accumulate here; you control what leaves |
| Isolate | `feat-dev/<feature>` | `feat-dev/trunk`     | pulls that one feature's commits in; local test only       |
| PR      | `feat/<feature>`     | `origin/main`        | cleaned-up subset → regular PR to upstream                 |

> `feat-dev/` is a **namespace**, not a bare branch. The trunk MUST be
> `feat-dev/trunk` (not `feat-dev`) so that `refs/heads/feat-dev/` is a
> directory and `feat-dev/<feature>` can exist. A bare branch named `feat-dev`
> makes the slash form git-illegal (`cannot lock ref ... 'refs/heads/feat-dev'
exists`).

## Data flow

```
origin/main ────────────────┐  upstream (read-only to you)
                            │
feat-dev/trunk  (everything you build)
        │  pull commits for one feature
        ▼
feat-dev/<feature>   (isolate + local test)
        │  cherry-pick ONLY clean/reviewable commits
        ▼
feat/<feature>  (base = their main, tidied) ──► push Fork ──► PR to origin/main
```

"Cleaned up" = start the Tier-2 branch at `origin/main` so the diff carries
NONE of your trunk history; bring in just the tidy commits. That is why Tier-2
forks from upstream, not from your trunk.

## Promote one feature upstream

```sh
cd ~/.hermes/hermes-agent
git switch -c feat-dev/<feature> feat-dev/trunk      # isolate + test locally
git switch -c feat/<feature> origin/main            # clean base, their main
git cherry-pick <clean shas from feat-dev/<feature>> # selective pull
#   └─ drop WIP / reword / squash = the "cleaned up" step
git push Fork feat/<feature>                         # then open PR against origin/main
```

## Pull upstream in ON YOUR TERMS

```sh
git fetch origin
git switch feat-dev/trunk
   ├─ selective:  git cherry-pick <upstream sha>
   └─ all:        git merge origin/main
```

You choose; upstream never forces itself onto your features.

## Guard existing open Fork PRs

```sh
feat/* branch with a live PR (e.g. feat/fix-nonlocal-final-args)
   ├─ DO NOT rebase / force-push it
   ├─ its upstream stays Fork/feat-dev/<...> or Fork/feat/<...> - leave it
   └─ new features branch from feat-dev/trunk → new Fork ref; old PRs stay isolated
```

## Setup (one-time)

```sh
cd ~/.hermes/hermes-agent
git branch -m feat-dev feat-dev/trunk # if a bare feat-dev exists
git branch feat-dev/trunk main        # if starting fresh
# verify: no upstream on the trunk
git rev-parse --abbrev-ref feat-dev/trunk@{upstream} # should FAIL (no upstream)
```

## `hermes update` does NOT wipe branches

`hermes update` only checks out `main` and fast-forwards `origin/main`/`main`
from NousResearch/hermes-agent. Your local branches (`feat-dev/trunk`,
`feat-dev/*`, `feat/*`) and the `Fork` remote are **untouched** - only `HEAD`
moves to `main`.

Consequence: after an update, `feat-dev/trunk` (based on an OLD `main`) has
silently diverged from the advanced upstream. Do NOT blindly merge. Deliberately
re-sync the trunk on your terms:

```sh
cd ~/.hermes/hermes-agent
git switch main && git fetch origin && git merge --ff-only origin/main # catch up main
git switch feat-dev/trunk
git merge origin/main # OR cherry-pick specific upstream commits
# then re-test each feat-dev/<feature> (rebase onto new trunk if needed)
```

Same "pull upstream in on your terms" rule, made explicit for the post-update
moment.

## Verify state (no mutations)

```sh
git branch                    # feat-dev/trunk, feat/*, main
git branch -a | grep feat-dev # confirms namespace + trunk
git remote -v                 # Fork/origin unchanged
```

## Hard rules of engagement

1. `feat-dev/trunk` NEVER tracks `origin/main`. It is your moving base.
2. Upstream enters only by explicit `cherry-pick`/`merge` you trigger.
3. `feat/<feature>` is the ONLY branch that touches upstream, via a normal PR.
4. Existing open Fork PRs are never rebased/force-pushed - they stay isolated.
5. No command in this skill pushes, fetches, or deletes a remote ref unless the
   user explicitly asks. **Location (origin of each branch) is recorded, not
   assumed.**
