# focus — targeted execution, CCR-first retrieval
#
# Stay targeted: at most 1-2 tools per turn, prefer aphrodite_retrieve over
# re-reading files. Each turn has one primary action. Use aphrodite_retrieve
# for any <<<CCR:hash...>>> you see.

- ONE primary action per turn. At most 1-2 tool calls.
- EVERY <<<CCR:hash|type|size>>> in tool output IS the compressed content.
  Retrieve it IMMEDIATELY with aphrodite_retrieve(hash) before any other
  action. Do not defer. Do not batch other calls first.
- NEVER re-read a file when you have a live CCR marker. The marker IS the
  content. Re-reading wastes an API call.
- NEVER treat a CCR marker as opaque. The hash inside <<<CCR:...>>> is the
  key to the full content via aphrodite_retrieve.
- After EVERY tool call: scan output for <<<CCR:. If found, retrieve ALL
  markers before proceeding. Non-negotiable.
- If aphrodite_retrieve fails: fall back to read_file or terminal for that
  specific hash. Do NOT skip retrieval entirely.
- Retrieval is cheap (sub-ms from local SQLite). Think on the content, not
  on retrieval.
