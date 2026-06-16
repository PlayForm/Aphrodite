# Headroom Submodule Forking Workflow

When headroom vendored code needs fixes, fork it and track via submodule.

## Setup

```bash
# 1. Fork headroom on GitHub (NikolaRHristov/headroom)
# 2. The submodule already has our fork as origin:
cd vendor/headroom
git remote -v  # origin → git@github.com:NikolaRHristov/headroom.git
               # upstream → https://github.com/chopratejas/headroom.git
```

## Workflow

```bash
# Pull latest upstream
cd vendor/headroom
git fetch upstream
git rebase upstream/main

# Make fixes, commit
git add -A
git commit -m "fix(headroom-proxy): description"
git push origin main

# Update parent repo submodule pointer
cd ../..
git add --force vendor/headroom
git commit -m "chore: update headroom submodule — description"
git push
```

## What NOT to do

- Don't edit vendored code in-place without committing to the fork first
- Don't push submodule pointer without committing inside the submodule first
- Don't modify headroom files expecting them to be tracked in the parent repo

## Key Fix Applied

**x-headroom-workspace header preserve**: In `crates/headroom-proxy/src/headers.rs`,
`is_internal_header()` was modified to exclude `x-headroom-workspace` from stripping.
This header carries the resolved workspace key for CCR cross-project scoping.
Without this fix, all CCR entries lose workspace scoping when going through headroom-proxy.
