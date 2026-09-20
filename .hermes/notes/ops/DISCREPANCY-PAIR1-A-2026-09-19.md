# DISCREPANCY-PAIR1-A-2026-09-19

Append-only discrepancy log for child A of PAIR-1 (docs/architecture/01-06).
Format: `- [date] file: claim X stale; source says Y (file:line)`.

- [2026-09-19] .hermes/uml/03-retrieve.md: claim "`limit == 0` is **not** unlimited - it clamps to a 10,000-line server cap (02-F5)" is stale; source says `paginate` maps `limit == 0` to `usize::MAX` = full document, and only explicit nonzero limits clamp to 10,000 lines (crates/aphrodite/src/retrieve.rs:194-196, 210-218; test comment :387-391).
- [2026-09-19] .hermes/uml/01-06: file:line anchors are systematically drifted vs Development HEAD (e.g. main() is main.rs:38 not :31; run() main.rs:107 not :98; proxy_handler proxy.rs:913 not :926; compress_chat_completion proxy.rs:2084 not :2103; register() **init**.py:1086 not :1106; aphrodite_hermes_call_hook lib.rs:307 not :297; build_state proxy.rs:643 not :656). Anchors are internal-only and were stripped from the public docs; no behavioral impact observed.
