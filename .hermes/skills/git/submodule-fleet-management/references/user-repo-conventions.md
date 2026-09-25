# User repo conventions (interpreting remote/branch state)

- **`Source`** is the user's canonical remote (e.g. `ssh://git@github.com/<User>/<Repo>.git`)
  — fetch/push target. A repo with only `Source` and no `origin` is normal.
- **`Parent`** exists ONLY for actual forks (points at the upstream fork-source, e.g.
  `ohmyzsh/ohmyzsh` for the ZSH repo). A host-only `Parent` (`ssh://git@github.com/.git`)
  is the empty-Parent bug — strip it.
- Working branch is **`Current`** (with `Previous` as its sibling); branches carry no
  upstream by default — push explicitly (`git push Source Current`) and verify with
  `git ls-remote Source Current`.
- A `Parent` remote with a broken URL (e.g. `ssh://git@github.com/.git`) is a copy-paste
  artifact, not a config the user relies on — removing it is safe.
- Dotfiles are symlinks into a dotfiles git repo — edits to `~/.zshrc`,
  `~/.bashrc` show up as repo modifications; the user commits them.
