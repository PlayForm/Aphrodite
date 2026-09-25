#!/bin/bash
# normalize-tabs.sh - Post-tool-call hook for the Aphrodite hook pipeline
# Replaces literal \t (backslash + t) with actual tab characters (0x09)
# after write_file or patch tool calls. Prevents the patch tool from
# corrupting file indentation in Aphrodite crate and plugin sources before
# they reach cargo fmt.
#
# The Hermes patch tool occasionally writes literal \t sequences instead
# of real tab characters when processing JSON-escaped tab content from
# new_string/old_string parameters. This hook catches and fixes those
# after every write_file or patch call.
#
# Sibling of normalize-dashes.sh - same JSON protocol, same placement
# in hooks.post_tool_call in config.yaml.
#
# stdin (received by script):
#   { "tool_name": "patch", "tool_input": { "path": "/path/to/file" }, ... }
#
# Claude Code note: uses .tool_input.path (Hermes), NOT .tool_input.file_path (Claude)

INPUT=$(cat)
FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.path // empty')

[ -z "$FILE_PATH" ] && exit 0
[ -f "$FILE_PATH" ] || exit 0
file --brief --mime-encoding "$FILE_PATH" 2>/dev/null | grep -q 'binary' && exit 0

# Replace literal \t at indentation positions with real tab characters.
# Anchored to start-of-line or after existing real tabs so we don't
# corrupt \t inside string literals or comments.
perl -i -pe '
  while (/^(?:\t|\\t)*\K\\t/) {
    my $pos = pos;
    substr($_, $-[0], 2) = "\t";
    pos = $pos - 1;
  }
' "$FILE_PATH" 2>/dev/null

exit 0