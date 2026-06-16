# Dash Normalization Audit - 2026-06-13

## Investigation Results

### Hook Works Manually
The perl one-liner correctly replaces U+2014 em dash with ASCII -. Tested.

### 50 Lines Across 32 Files Affected
All U+2014 em dash, all in Documentation/GitHub/README.md, Architecture.md, and
Documentation/Rust/README.md files. Pattern: "None - zero overhead".

### Introduced by Subagent Work
Git blame shows commit bfefeb33 (2026-06-13 01:34 UTC+3) introduced 24 dashes
in Wind alone. Parent commit was clean. Work done by delegate_task subagents.

### Why Hook Failed
post_tool_call hooks do not fire reliably in delegate_task subagent sessions.
See skill subagent-hook-reliability for details.

## Quick Fix Command
find Land -name '*.md' -exec perl -i -CSD -pe 's/[\x{2014}]/-/g' {} +