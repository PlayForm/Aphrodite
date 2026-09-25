---
name: github-issues
description: "Use when creating, triaging, or managing GitHub issues. List, label, assign, comment, close via gh or REST for the Aphrodite repos."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-issues
date: 2026-09-25
metadata:
    hermes:
        tags: [github, issues, project-management, bug-tracking, triage]
        related_skills: [github-issue-responses, github-pr-workflow]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - The issue CRUD and triage flows (gh and REST)
    - The bug/feature body templates
depends_on:
    - github-auth
supersedes: []
---

# GitHub Issues

Create, search, triage, and manage GitHub issues on the PlayForm org repos.
Each operation shows `gh` first, then the `curl` fallback.

## When to Use

- User asks to file, list, or view issues
- Triage backlog, apply labels, assign owners
- Close/reopen or bulk-manage issues

## Prerequisites

- Authenticated with GitHub - see `github-auth` skill
- Inside a git repo with a GitHub remote (or pass the repo explicitly,
  e.g. `--repo PlayForm/Aphrodite`)

## Setup

```bash
source "${HERMES_HOME:-$HOME/.hermes}/skills/github/github-auth/scripts/gh-env.sh"
```

Sets `GH_AUTH_METHOD`, `GITHUB_TOKEN`, `GH_OWNER`, `GH_REPO`.

## View Issues

```bash
gh issue list --repo PlayForm/Aphrodite
gh issue list --state open --label "bug"
gh issue list --assignee @me
gh issue list --search "CCR marker" --state all
gh issue view 42
```

Without gh: `GET /repos/{o}/{r}/issues` (add
`?state=open&labels=bug&per_page=20` to filter), search via
`GET /search/issues?q=...`. Filter `pull_request` entries out - the issues
endpoint returns PRs too.

## Create Issues

```bash
gh issue create \
	--title "Raw CCR marker appears in assistant output" \
	--body "## Description\n...\n## Steps to Reproduce\n...\n## Expected Behavior\n..." \
	--label "bug,proxy" \
	--assignee "username"
```

Without gh: `POST /repos/{o}/{r}/issues` with `{"title", "body", "labels":
[...], "assignees": [...]}`.

Use `templates/bug-report.md` and `templates/feature-request.md` for
structured bodies.

## Manage Issues

Labels:

```bash
gh issue edit 42 --add-label "priority:high,bug"
gh issue edit 42 --remove-label "needs-triage"
```

Without gh: `POST /repos/{o}/{r}/issues/42/labels` with `{"labels": [...]}`,
`DELETE /repos/{o}/{r}/issues/42/labels/{name}`, and list repo labels via
`GET /repos/{o}/{r}/labels`.

Assignees:

```bash
gh issue edit 42 --add-assignee username # or @me
```

Without gh: `POST /repos/{o}/{r}/issues/42/assignees` with
`{"assignees": ["username"]}`.

Comments:

```bash
gh issue comment 42 --body "Root cause is in the proxy crate."
```

Without gh: `POST /repos/{o}/{r}/issues/42/comments` with `{"body": "..."}`.

Close/reopen:

```bash
gh issue close 42 --reason "not planned"
gh issue reopen 42
```

Without gh: `PATCH /repos/{o}/{r}/issues/42` with `{"state": "closed",
"state_reason": "completed"}` or `{"state": "open"}`.

Link to PRs: `Closes #42` / `Fixes #42` / `Resolves #42` in a PR body
auto-closes the issue on merge.

Branch from an issue: `gh issue develop 42 --checkout`, or manually
`git checkout Development && git pull origin Development && git checkout -b fix/issue-42-ccr-marker`.

## Triage Workflow

1. List untriaged: `gh issue list --label "needs-triage" --state open`
   (curl: `GET /repos/{o}/{r}/issues?labels=needs-triage&state=open`, filter
   out PRs)
2. Read and categorize each issue
3. Apply labels and priority (see Manage Issues)
4. Assign when the owner is clear
5. Comment with triage notes

## Bulk Operations

```bash
gh issue list --label "wontfix" --json number --jq '.[].number' \
	| xargs -I {} gh issue close {} --reason "not planned"
```

Without gh: list numbers via
`GET /repos/{o}/{r}/issues?labels=wontfix&state=open`, then loop
`PATCH /repos/{o}/{r}/issues/$num` with `{"state": "closed",
"state_reason": "not_planned"}` per number.

## Quick Reference

| Action  | gh                               | curl endpoint                            |
| ------- | -------------------------------- | ---------------------------------------- |
| List    | `gh issue list`                  | `GET /repos/{o}/{r}/issues`              |
| View    | `gh issue view N`                | `GET /repos/{o}/{r}/issues/N`            |
| Create  | `gh issue create`                | `POST /repos/{o}/{r}/issues`             |
| Labels  | `gh issue edit N --add-label`    | `POST /repos/{o}/{r}/issues/N/labels`    |
| Assign  | `gh issue edit N --add-assignee` | `POST /repos/{o}/{r}/issues/N/assignees` |
| Comment | `gh issue comment N --body`      | `POST /repos/{o}/{r}/issues/N/comments`  |
| Close   | `gh issue close N`               | `PATCH /repos/{o}/{r}/issues/N`          |
| Search  | `gh issue list --search`         | `GET /search/issues?q=...`               |

## Pitfalls

- Filter `pull_request` entries out of `/issues` responses - the endpoint
  returns PRs and they corrupt counts and bulk loops.
- Use `state_reason: "not_planned"` (or `--reason "not planned"`) when
  closing wontfix - a bare state change loses triage meaning.
- Use `templates/bug-report.md` for bug bodies - a structured reproduction
  beats prose.
- Include `Closes #N` in PR bodies to auto-close issues on merge - forgetting
  it orphans the issue.
- For bulk loops, always filter by label AND state or you may close issues
  you did not intend.
