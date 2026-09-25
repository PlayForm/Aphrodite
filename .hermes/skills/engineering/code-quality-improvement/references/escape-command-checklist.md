# Escape Command Checklist

## POSIX Native Commands to Escape

All of these should be prefixed with `\` when they appear at command position:

### File navigation

`pwd` `cd` `ls` `dirname` `basename` `realpath` `readlink` `stat`

### File manipulation

`cp` `mv` `rm` `mkdir` `rmdir` `touch` `chmod` `chown` `ln` `install` `mktemp`

### Text processing

`echo` `printf` `read` `cat` `grep` `sed` `awk` `sort` `uniq` `wc` `head` `tail` `cut` `tr` `tee` `diff` `patch` `comm` `cmp`

### Shell builtins

`exit` `export` `source` `.` `exec` `shift` `unset` `type` `eval` `set` `trap` `command` `test` `wait` `kill` `read` `printf` `times` `ulimit` `umask`

### Process/scheduling

`sleep` `time` `nohup` `seq` `shuf` `true` `false` `env` `which` `wait`

### Search

`find` `xargs`

## Command Position Detection

A token is a COMMAND (not an argument) when immediately preceded by:

| Preceding context                         | Example             |
| ----------------------------------------- | ------------------- |
| Start of line (after optional whitespace) | `\t\techo "hello"`  |
| Pipe `\|`                                 | `x \| \echo y`      |
| Pipe-with-stderr `\|&`                    | `x \|& \echo y`     |
| Logical OR `\|\|`                         | `a \|\| \echo fail` |
| Logical AND `&&`                          | `a && \echo ok`     |
| Semicolon `;`                             | `a; \echo b`        |
| Opening paren `(`                         | `(\echo hi)`        |
| Opening brace `{`                         | `{ \echo hi; }`     |
| `$(` command substitution                 | `$(\echo hi)`       |
| Backtick `` ` ``                          | `` `\echo hi` ``    |
| After `then`, `else`, `do`                | `then \echo hi`     |

## False Positives to Watch For

### Package names in JSON strings

These are ASCII strings inside JSON, NOT commands - do not escape:

- `"test"`, `"type"`, `"env"`, `"source"`, `"head"`, `"sort"`, `"install"`
- `"@playwright/test"`, `"@babel/preset-env"`, `"@vueuse/head"`, `"prettier-plugin-sort-imports"`

### Variable assignments with `true`/`false`

```bash
Flag=true  # ← "true" is a value, not a command - skip
Loop=false # ← "false" is a value, not a command - skip
```

### `$()` vs `$(())`

```bash
# $(()) is arithmetic expansion - commands inside need escaping
Count=$(( $(wc -l < file) + 1 ))   # wc is a command - escape it
Count=$(\\( $(\\wc -l < file) + 1 ))  # ✅ correct
```

### Already-escaped commands

If a command already has `\\` (e.g., `\\cd`, `\\echo`), do NOT add another one. Double-escaping (`\\\\cd`) looks for an executable named `\\cd` which doesn't exist.

### Generated/archival files

Generated/archival `.sh` trees (outside the repo tree) are full of repeated `cd "..." || exit` patterns. These can be batch-fixed with `patch(replace_all=true)` on the exact pattern `|| exit` → `|| \\exit` and `cd "$` → `\\cd "$`. But be careful - check that no lines already have `\\cd` before using replace_all.

## Do NOT escape external tools

`git`, `gh`, `jq`, `node`, `npm`, `pnpm`, `yarn`, `cargo`, `rustc`, `python`, `bash`, `sh`, `docker`, `sort-package-json`, `mapfile`

## Verification

After escaping, verify with a simple scan:

```bash
# Check for remaining unescaped commands at line-start (true positives only)
grep -rn '^\s*pwd\b' --include='*.sh' .
grep -rn '^\s*echo\b' --include='*.sh' .
grep -rn '^\s*exit\b' --include='*.sh' .
grep -rn '^\s*export\b' --include='*.sh' .
# etc.
```

Each result should be manually reviewed to confirm it's NOT a false positive (JSON string, package name, comment, etc.).
