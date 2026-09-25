---
name: github-pr-workflow
description: "Use when driving the GitHub PR lifecycle. Branch, commit, open, monitor CI, merge via gh or REST for the Aphrodite monorepo."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-pr-workflow
date: 2026-09-25
metadata:
    hermes:
        tags: [github, pull-requests, ci, git, automation, merge]
        related_skills: [github-auth, github-code-review, github-batch-pr-merge]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - The full PR lifecycle (branch, commit, open, CI, merge)
    - The org-wide batch-sweep cross-reference
depends_on:
    - github-auth
    - github-code-review
supersedes: []
---

# GitHub Pull Request Workflow

Manage the full PR lifecycle: branch, commit, open, monitor CI, fix
failures, merge. Each section shows `gh` first, then the `git` + `curl`
fallback. Base branch for the Aphrodite monorepo is `Development`.

## When to Use

- User asks to open a PR for current changes
- Monitor or fix CI on a PR branch
- Merge or auto-merge a PR

## Prerequisites

- Authenticated with GitHub - see `github-auth` skill
- Inside a git repository with a GitHub remote

## Setup

```bash
source "${HERMES_HOME:-$HOME/.hermes}/skills/github/github-auth/scripts/gh-env.sh"
```

Sets `GH_AUTH_METHOD`, `GITHUB_TOKEN`, `GH_OWNER`, `GH_REPO`.

## 1. Branch

```bash
git fetch origin && git checkout Development && git pull origin Development
git checkout -b feat/ccr-marker-retrieval
```

Conventions: `feat/`, `fix/`, `refactor/`, `docs/`, `ci/` prefixes.

## 2. Commit

```bash
git add crates/aphrodite/src crates/aphrodite/tests
git commit -m "feat: add retrieval query support to CCR markers

- Add query narrowing to the retrieve path
- Add unit tests for marker expansion"
```

Use Conventional Commits format (see `references/conventional-commits.md`):
`type(scope): short description`, types `feat`/`fix`/`refactor`/`docs`/
`test`/`ci`/`chore`/`perf`.

## 3. Push and Open the PR

```bash
git push -u origin HEAD

gh pr create \
	--title "feat: add retrieval query support to CCR markers" \
	--body "## Summary\n- ...\n## Test Plan\n- [ ] cargo test passes\n\nCloses #42" \
	--draft --reviewer user1,user2 --label "enhancement" --base Development
```

Options: `--draft`, `--reviewer user1,user2`, `--label "enhancement"`,
`--base Development`. Use `templates/pr-body-feature.md` /
`templates/pr-body-bugfix.md` for bodies; `Closes #42` auto-closes the
linked issue on merge.

Without gh: `POST /repos/{o}/{r}/pulls` with `{"title", "body", "head":
"<branch>", "base": "Development"}` (`"draft": true` for draft). The response
contains the PR `number` - save it for later commands.

## 4. Monitor CI

```bash
gh pr checks         # one-shot
gh pr checks --watch # poll until all checks finish
```

The Aphrodite repos run their CI from `Build.yml` (4-target matrix, 12
release assets) and release checks from `Publish.yml` (publish chain
headroom-core → aphrodite → aphrodite-hermes, tags `Aphrodite/v*`); failed
runs appear under the workflow name in `gh pr checks`.

Without gh: `GET /repos/{o}/{r}/commits/<sha>/status` (combined status) and
`/check-runs` (Actions runs). Poll loop: check `state` every 30s, break on
success/failure/error, up to 20 attempts.

## 5. Fix CI Failures

```bash
gh run list --branch $(git branch --show-current) --limit 5
gh run view < RUN_ID > --log-failed
```

Without gh: `GET /repos/{o}/{r}/actions/runs?branch=<b>&per_page=5` to list
runs, then download failed logs via
`GET /repos/{o}/{r}/actions/runs/<id>/logs` (zip - unzip and read). See
`references/ci-troubleshooting.md`.

Auto-fix loop: check status -> read failing logs -> fix with
`patch`/`write_file` -> `git add` + commit + push -> wait for CI -> re-check.
Repeat up to 3 attempts, then ask the user.

## 6. Merge

```bash
gh pr merge --squash --delete-branch
gh pr merge --auto --squash --delete-branch # merge when all checks pass
```

Without gh: `PUT /repos/{o}/{r}/pulls/<n>/merge` with
`{"merge_method": "squash", "commit_title": "..."}` (methods: `merge`,
`squash`, `rebase`), then delete the remote branch
(`git push origin --delete <b>`), checkout Development, pull, and delete the
local branch.

Auto-merge requires GraphQL - REST lacks it: get the PR node ID from
`GET /repos/{o}/{r}/pulls/<n>` (`node_id`), then `POST /graphql` with
`mutation { enablePullRequestAutoMerge(input: {pullRequestId: "...",
mergeMethod: SQUASH}) { clientMutationId } }`. The repo must have auto-merge
enabled in settings.

## Quick Reference

| Action         | gh                                 | git + curl                                               |
| -------------- | ---------------------------------- | -------------------------------------------------------- |
| List my PRs    | `gh pr list --author @me`          | `GET /repos/{o}/{r}/pulls?state=open`                    |
| View diff      | `gh pr diff`                       | `git diff Development...HEAD`                            |
| Comment        | `gh pr comment N --body`           | `POST /repos/{o}/{r}/issues/N/comments`                  |
| Request review | `gh pr edit N --add-reviewer user` | `POST /repos/{o}/{r}/pulls/N/requested_reviewers`        |
| Close PR       | `gh pr close N`                    | `PATCH /repos/{o}/{r}/pulls/N`                           |
| Check out PR   | `gh pr checkout N`                 | `git fetch origin pull/N/head:pr-N && git checkout pr-N` |

## Org-Wide Batch Sweep (merge all, close conflicts)

Full procedure in `github-batch-pr-merge`; the essentials:

1. Enumerate: `gh api 'search/issues?q=is:pr+is:open+org:ORG&per_page=100&page=N'`
    - paginate; **always check `.total_count`** (results cap at 100/page and
      truncation is silent). `gh search prs` uses `--owner`, NOT `--org`.
2. Build the list as `OWNER/REPO<TAB>number` and parse with
   `IFS=$'\t' read -r repo num`. **Never `IFS=/`** on `OWNER/REPO/N` - it
   splits into wrong fields and every `gh pr merge` fails with "expected the
   [HOST/]OWNER/REPO format" (the whole batch fails without touching
   anything).
3. Mergeability is stale-able: a PR can report MERGEABLE/CLEAN in the list
   yet the merge attempt fails with "Pull Request has merge conflicts". The
   **merge attempt itself is the authoritative check** - try
   `gh pr merge --squash --delete-branch`; on failure, close only if the
   error mentions merge conflict; log-and-skip anything else (don't close
   PRs that failed for other reasons).
4. Run big sweeps (100+ PRs) as a background script with
   `notify_on_complete=true` - a few hundred API calls takes minutes.
5. Re-verify with the same search query; expect `total_count` 0. Dependabot
   may open new PRs mid-sweep - re-check and handle stragglers.
6. Scratch list/log/script go under `~/.hermes/tmp/` and stay there.

## Pitfalls

- Merge only a branch that is up to date with Development - squash-merging a
  stale branch silently reverts recent fixes.
- Include `Closes #N` in the PR body to auto-close the linked issue on merge.
- Auto-merge via GraphQL requires TWO preconditions: (1) the repo setting
  `allow_auto_merge` must be true, AND (2) the base branch must have branch
  protection rules requiring >=1 status check or review. Without protection,
  `enablePullRequestAutoMerge` fails with `Pull request Protected branch
rules not configured for this branch (enablePullRequestAutoMerge)`.
- `gh pr merge --auto` only calls the auto-merge mutation when the PR is NOT
  immediately mergeable - gh CLI merges directly (no auto-merge, no
  protection needed) when mergeStateStatus is CLEAN/UNSTABLE/HAS_HOOKS (see
  `isImmediatelyMergeable` in gh source). So the failure only surfaces when
  the PR is BEHIND/BLOCKED - e.g. the 2nd dependabot PR in a batch whose
  sibling merged first.
- Dependabot batch PRs: the first PR direct-merges; later siblings go BEHIND
  -> auto-merge path -> fails on any unprotected branch. Symptom: Approve
  job green, Merge job red with the enablePullRequestAutoMerge error.
- Branch protection on PRIVATE repos is 403-blocked on GitHub's Free plan
  (`Upgrade to GitHub Pro or make this repository public to enable this
feature`) - public repos only, or paid plan.
- When enabling branch protection batch-wide: require a check that actually
  runs on that repo. Requiring a check context that never runs (e.g.
  `Assign` on a repo without the Build.yml workflow) permanently locks all
  merges. Repos without the workflow -> use a 1-review requirement instead;
  both satisfy the auto-merge precondition.
- Auto-merge is impossible via REST - use the GraphQL mutation or `gh`.
- Transient API failures (HTTP 401/429/5xx) surface on long batch runs -
  retry the failed item individually with backoff, and re-dispatch failed
  clusters in smaller chunks rather than re-running the whole sweep.
- After merging, delete the branch remotely and locally to avoid stale refs.
- Draft PRs skip reviewers until marked ready - mark ready before requesting
  review.
