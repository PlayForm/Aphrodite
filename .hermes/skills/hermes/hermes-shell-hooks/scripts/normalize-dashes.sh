#!/bin/bash
# normalize-dashes.sh - Post-tool-call hook for the Aphrodite hook pipeline
# Replaces Unicode dash characters with plain ASCII hyphen-minus (0x2D)
# after write_file or patch tool calls. Prevents em-dashes (-), en-dashes (-),
# and other Unicode dash variants from contaminating code files in the
# Aphrodite monorepo (crate sources, plugin Python, TOML manifests).
#
# Ported from Claude Code hook - note the field name difference:
# Hermes uses `.tool_input.path`, Claude uses `.tool_input.file_path`.

INPUT=$(cat)
FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.path // empty')

# Skip if no path provided
[ -z "$FILE_PATH" ] && exit 0

# Skip if file doesn't exist
[ -f "$FILE_PATH" ] || exit 0

# Skip binary files
file --brief --mime-encoding "$FILE_PATH" 2>/dev/null | grep -q 'binary' && exit 0

# Replace all Unicode dash variants with ASCII hyphen
perl -i -CSD -pe \
  's/[\x{058A}\x{05BE}\x{1400}\x{1806}\x{2010}-\x{2015}\x{2E17}\x{2E1A}\x{2E3A}-\x{2E3B}\x{2E40}\x{2E5D}\x{301C}\x{3030}\x{30A0}\x{FE31}-\x{FE32}\x{FE58}\x{FE63}\x{FF0D}]/-/g' \
  "$FILE_PATH" 2>/dev/null

exit 0