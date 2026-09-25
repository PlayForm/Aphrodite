# Converting plain directories into submodules (dirs → own repos → gitlinks)

Reference for SKILL.md ("Converting existing plain directories into submodules").
Full walkthrough; the rule itself lives in SKILL.md. All commands are byte-stable.

When repo content that lived as plain dirs must become submodule-crates (each dir = own
repo, superproject tracks gitlinks), do it in this order and gate every push on integrity:

1. Per crate: `git init -q -b Current`, commit the EXISTING content first (the gitlink
   replaces the tree - content must be committed before conversion), add the remote, push.
2. Superproject: `git rm -r --cached <path>` per crate (working trees stay), write
   `.gitmodules` with `git config -f .gitmodules submodule.<path>.<key> <value>` (never
   hand-edit the file - reason UNKNOWN), `git add <path>` (gitlink - use `-f` when the
   entry has `ignore = all`), then `git submodule init` + `git submodule sync`.
3. Hydrate with `git submodule update --init --recursive`. Freshly-hydrated submodules
   come out DETACHED at the recorded gitlink - `git submodule status` shows `(sha)`
   instead of `(heads/Current)`; re-attach with `git -C <sub> checkout <branch>`.
4. Integrity gate before any push: for EVERY sub, recorded gitlink (`git ls-tree HEAD
<sub>` / `git ls-files -s -- <sub>`) == `git -C <sub> rev-parse HEAD`; per-crate
   tracked-file counts match the pre-conversion listing; the key source file non-empty
   with the expected header (`#![allow(non_snake_case)]`); the crate manifest contains the
   crate name; root history still holds the pre-conversion files.
5. A submodule branch that advances AFTER registration (e.g. a cleanup push) shows `+` in
   `git submodule status` - that is a gitlink bump in the superproject (`git add -f <sub>`
    - commit per the repo's commit cadence), never a revert of the sub.
6. The wipe-and-restore path (`mv <live> <Backup>`, fresh `git clone
--recurse-submodules`) is valid ONLY after the root's gitlinks AND every submodule
   branch are pushed: verify `git ls-remote <root-url>` and per-sub `git ls-remote
<sub-url> <branch>` reach the intended heads BEFORE recommending the wipe, then run the
   fresh clone's `git submodule status` (zero `+`/`-`, every entry `(heads/<branch>)`) as
   the restore gate.
