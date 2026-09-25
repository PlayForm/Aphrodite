---
name: github-auth
description: "Use when GitHub authentication is missing or broken. HTTPS tokens, SSH keys, or gh CLI login for PlayForm org and public repo work."
version: 1.3.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-auth
date: 2026-09-25
metadata:
    hermes:
        tags: [github, authentication, setup, login, token, ssh]
        related_skills: [github-pr-workflow, github-issues, github-repo-management]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - The auth decision tree (gh vs token vs SSH)
    - The shared gh-env.sh detection helper
depends_on: []
supersedes: []
---

# GitHub Authentication

Set up authentication for working with GitHub repos, PRs, issues, and CI on
the PlayForm org and public repos. Two paths: `git` with HTTPS tokens or SSH
keys (always available, no sudo), and `gh` CLI (richer API access - the
standard for Aphrodite workflows).

## When to Use

- GitHub commands fail with permission or authentication errors
- `gh auth status` reports not logged in
- Setting up a fresh machine for GitHub work
- A workflow needs repo/org-level API access (PRs, issues, CI, releases) on
  PlayForm/Aphrodite or PlayForm/Aphrodite-Hermes

## Detect Current State

```bash
git --version
gh --version 2> /dev/null || echo "gh not installed"
gh auth status 2> /dev/null || echo "gh not authenticated"
```

Decision tree: `gh auth status` OK -> use `gh` everywhere; `gh` installed but
unauthenticated -> run `gh auth login`; no `gh` -> use the git-only methods
below.

## Method 1: HTTPS Personal Access Token (git only)

1. Create a classic token at https://github.com/settings/tokens with scopes
   `repo` (full repo access), `workflow` (Actions), and `read:org` (org
   repos), plus an expiry. Repo work needs `repo` + `workflow`; org-wide
   sweeps (batch PR merges, fork farms) additionally need `read:org`.
2. Configure git to store it:

```bash
git config --global credential.helper store
# First operation prompts: username = GitHub username, password = the token (NOT the account password)
git ls-remote https://github.com/ < username > / < any-repo > .git
```

Alternatives: `git config --global credential.helper 'cache --timeout=28800'`
(memory, 8h), or embed per-repo:
`git remote set-url origin https://<user>:<token>@github.com/<owner>/<repo>.git`.

3. Set identity:

```bash
git config --global user.name "Name"
git config --global user.email "email@example.com"
```

4. Verify: `git ls-remote https://github.com/<username>/<any-repo>.git`
   completes without prompts.

## Method 2: SSH Key (git only)

```bash
ls -la ~/.ssh/id_*.pub 2> /dev/null || echo "No SSH keys found"
ssh-keygen -t ed25519 -C "email@example.com" -f ~/.ssh/id_ed25519 -N ""
cat ~/.ssh/id_ed25519.pub # paste at https://github.com/settings/keys
ssh -T git@github.com     # expect "Hi <user>! You've successfully authenticated"
git config --global url."git@github.com:".insteadOf "https://github.com/"
git config --global user.name "Name"
git config --global user.email "email@example.com"
```

## Method 3: gh CLI

```bash
gh auth login                               # interactive browser login
echo "<TOKEN>" | gh auth login --with-token # headless / SSH server
gh auth setup-git                           # wire git credentials through gh
gh auth status                              # verify
```

`gh auth refresh -h github.com -s delete_repo` adds scopes (device flow)
when a later operation needs repo deletion (see `github-repo-management`).

## API Access Without gh

```bash
export GITHUB_TOKEN="<token>"
curl -s -H "Authorization: token ***" https://api.github.com/user
```

Recover a token from the git credential store:

```bash
grep "github.com" ~/.git-credentials | head -1 | sed 's|https://[^:]*:\([^@]*\)@.*|\1|'
```

## Shared Environment Script

All GitHub skills source the canonical detection script instead of
duplicating setup. It sets `GH_AUTH_METHOD` (gh/curl/none), `GITHUB_TOKEN`,
`GH_USER`, `GH_OWNER`, `GH_REPO`:

```bash
source "${HERMES_HOME:-$HOME/.hermes}/skills/github/github-auth/scripts/gh-env.sh"
```

Inside the Aphrodite repo the same helper resolves `GH_OWNER=PlayForm`,
`GH_REPO=Aphrodite` from the `origin` remote. The repo's canonical remote is
named `Source` (`ssh git@github.com/PlayForm/Aphrodite.git`) - clone with
`--origin Source` plus an `origin` alias, or pass `--repo PlayForm/Aphrodite`
explicitly, so the helper's `origin` lookup resolves.

## Pitfalls

- Never use a GitHub account password as the git password - GitHub disabled
  password auth; use a token.
- A `remote: Permission to X denied` push error usually means the token lacks
  `repo` scope - regenerate with the correct scopes. Pushing branches that
  contain `.github/workflows/*` additionally requires `workflow` scope.
- After stale cached credentials fail, run `git credential reject` then
  re-authenticate.
- When SSH port 22 is blocked, route through 443: add `Host github.com` with
  `Hostname ssh.github.com` and `Port 443` to `~/.ssh/config`.
- Keep the credential helper set to `store` or `cache` - otherwise
  credentials never persist.
- For multiple GitHub accounts use per-host SSH keys in `~/.ssh/config` or
  per-repo credential URLs.
- A token that is defined but not exported (or commented out in its env file)
  fails later with no obvious cause - after any setup, verify the live
  session with `gh auth status` or a real `git ls-remote` call; never assume
  the config file's presence means the session works.
