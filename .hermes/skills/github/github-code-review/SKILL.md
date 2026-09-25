---
name: github-code-review
description: "Use when reviewing code or pull requests. Diff local changes, post inline comments, and submit reviews via gh or REST for the Aphrodite monorepo."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-code-review
date: 2026-09-25
metadata:
    hermes:
        tags: [github, code-review, pull-requests, git, quality]
        related_skills: [github-auth, github-pr-workflow]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - The review checklist and verdict rule
    - The inline-comment + summary-comment flow (gh and REST)
depends_on:
    - github-auth
supersedes: []
---

# GitHub Code Review

Review local changes before pushing, or review open PRs. Plain `git` covers
local review; the `gh`/`curl` split only matters for PR-level interactions.
Applies to PRs against the Aphrodite monorepo (`PlayForm/Aphrodite`); the
plugin ships as the `plugins/aphrodite` submodule (remote `Source`, alongside
`vendor/headroom` and `vendor/rtk`) and the base branch is `Development`.

## When to Use

- User asks to review code before pushing
- User asks to review a PR (by number or URL)
- User wants inline comments or an approve/request-changes verdict

## Prerequisites

- Authenticated with GitHub - see `github-auth` skill
- Inside a git repository with a GitHub remote

## Setup

```bash
source "${HERMES_HOME:-$HOME/.hermes}/skills/github/github-auth/scripts/gh-env.sh"
```

Sets `GH_AUTH_METHOD` (gh/curl), `GITHUB_TOKEN`, `GH_OWNER`, `GH_REPO`.

## 1. Review Local Changes (Pre-Push)

```bash
git diff --staged                       # staged changes
git diff Development...HEAD             # all changes vs the base branch
git diff Development...HEAD --name-only # changed files
git diff Development...HEAD --stat      # insertions/deletions per file
git log Development..HEAD --oneline     # commit list
```

Scan for common issues:

```bash
git diff Development...HEAD | grep -nE "print\(|console\.log|TODO|FIXME|HACK|XXX|debugger"
git diff Development...HEAD | grep -inE "password|secret|api_key|token.*=|private_key"
git diff Development...HEAD | grep -nE "<<<<<<|>>>>>>|======="     # conflict markers
git diff Development...HEAD --stat | sort -t'|' -k2 -rn | head -10 # largest files
```

Review file by file with `read_file` for surrounding context. For the
Aphrodite repo, pay extra attention to changes under `crates/aphrodite`
(proxy), `crates/aphrodite-hermes` (Hermes bridge), and `plugins/aphrodite`
(the plugin surface). Present findings in the structure from
`references/review-output-template.md` (Critical / Warnings / Suggestions /
Looks Good).

## 2. Review a Pull Request

### Gather context

```bash
gh pr view 123
gh pr diff 123 --name-only
gh pr checks 123
```

Without gh: `GET /repos/{o}/{r}/pulls/123` (title, author, branches, body)
and `GET /repos/{o}/{r}/pulls/123/files` (changed files with
additions/deletions), both with `Authorization: token ***

### Check out the PR locally

```bash
gh pr checkout 123                                           # with gh
git fetch origin pull/123/head:pr-123 && git checkout pr-123 # plain git
git diff Development...pr-123
```

### Post an inline comment

```bash
HEAD_SHA=$(gh pr view 123 --json headRefOid --jq '.headRefOid')
gh api repos/$GH_OWNER/$GH_REPO/pulls/123/comments --method POST \
	-f body="Consider handling the error case here." \
	-f path="crates/aphrodite/src/proxy.rs" -f commit_id="$HEAD_SHA" -f line=45 -f side="RIGHT"
```

Without gh: `POST /repos/{o}/{r}/pulls/123/comments` with the same fields
(`body`, `path`, `commit_id`, `line`, `side`); get the head SHA from
`GET /repos/{o}/{r}/pulls/123` (`head.sha`).

### Submit a formal review

```bash
gh pr review 123 --approve --body "LGTM"
gh pr review 123 --request-changes --body "See inline comments."
gh pr review 123 --comment --body "Non-blocking suggestions."
```

Without gh, `POST /repos/{o}/{r}/pulls/123/reviews` submits multiple
comments atomically:

```bash
HEAD_SHA=$(curl -s -H "Authorization: token ***" \
	https://api.github.com/repos/$GH_OWNER/$GH_REPO/pulls/123 \
	| python3 -c "import sys,json; print(json.load(sys.stdin)['head']['sha'])")

curl -s -X POST -H "Authorization: token ***" \
	https://api.github.com/repos/$GH_OWNER/$GH_REPO/pulls/123/reviews \
	-d "{
    \"commit_id\": \"$HEAD_SHA\",
    \"event\": \"REQUEST_CHANGES\",
    \"body\": \"## Review\n2 issues, 1 suggestion.\",
    \"comments\": [
      {\"path\": \"crates/aphrodite/src/proxy.rs\", \"line\": 45, \"body\": \"Validate the input before it reaches the engine.\"},
      {\"path\": \"crates/aphrodite/src/ccr.rs\", \"line\": 23, \"body\": \"Return an error instead of panicking here.\"}
    ]
  }"
```

`event` values: `APPROVE`, `REQUEST_CHANGES`, `COMMENT`. The `line` field is
the line number in the NEW version of the file; use `"side": "LEFT"` for
deleted lines.

### Post a summary comment

After inline comments, always leave a top-level summary (template in
`references/review-output-template.md`):

```bash
gh pr comment 123 --body "$(
	cat << 'EOF'
## Code Review Summary

**Verdict: Changes Requested** (2 issues, 1 suggestion)

### 🔴 Critical
- **crates/aphrodite/src/proxy.rs:45** - unvalidated input reaches the engine.

### ⚠️ Warnings
- **crates/aphrodite/src/ccr.rs:23** - panic on malformed marker instead of an error path.

### 💡 Suggestions
- **crates/aphrodite/src/lib.rs:8** - duplicated logic, consider consolidating.

### ✅ Looks Good
- Clean error handling in the hermes bridge, good test coverage.
EOF
)"
```

### Clean up

```bash
git checkout Development && git branch -D pr-123
```

## 3. Review Checklist

Apply systematically to every review:

- **Correctness** - behavior matches claims; edge cases (empty input, nulls,
  concurrency) handled; error paths graceful
- **Security** - no hardcoded secrets; input validation; no SQLi/XSS/path
  traversal; authz checks present
- **Code quality** - clear naming; no premature abstraction; DRY;
  single-responsibility functions
- **Testing** - new paths covered; happy path and error cases; tests readable
  and maintainable
- **Performance** - no N+1 queries; caching where beneficial; no blocking
  calls in async paths
- **Documentation** - public APIs documented; non-obvious logic commented;
  README updated

## 4. End-to-End PR Review

1. Source gh-env.sh (Setup above)
2. Gather context: `gh pr view N`, `gh pr diff N --name-only`, `gh pr checks N`
3. Check out locally: `gh pr checkout N`
4. Read the diff file by file, then `read_file` for full context around changes
5. Run tests/lint locally if configured (`cargo test`, `cargo clippy` at the
   workspace root for crate changes)
6. Apply the checklist (Section 3)
7. Post the formal review + summary comment (Section 2)
8. Clean up: `git checkout Development && git branch -D pr-N`

Verdict rule: approve only with no critical or warning issues; request
changes on any critical/warning; comment when unsure or the PR is a draft.

## Pitfalls

- Review with surrounding context, not diffs alone - `read_file` the changed
  files or issues visible only in context get missed.
- Only approve when no critical or warning issues remain - minor suggestions
  are not blocking.
- For inline comments on deleted lines set `"side": "LEFT"` - the default
  refers to the new file.
- Comment inline first, then post the top-level summary - the author misses
  issues buried in review threads.
- Delete the local PR branch after review to avoid stale checkouts.
- Submit multi-comment reviews atomically via the `/reviews` endpoint -
  separate comment POSTs are not tied to a verdict.
- For Aphrodite PRs, an auto-committer may sweep the working tree mid-review;
  re-run `git status` before cleanup and never `git reset`/checkout to erase
  changes (`aphrodite-boundaries` git repair taxonomy).
