# DISCREPANCY-PAIR8-2026-09-19 - guides/ migration findings

Append-only log for PAIR-8 (docs/guides/ migration). Each entry is a claim
verified stale against source on 2026-09-19 and corrected in the rewrite.

- [2026-09-19] docs/hermes-integration.md:20: "All five hooks register in
  plugin.yaml" stale; the plugin registers SIX hooks
  (plugins/aphrodite/plugin.yaml:7-13 provides_hooks;
  plugins/aphrodite/**init**.py:1144-1175 registers one callback per name
  from aphrodite_hermes_get_hooks) - pre_tool_call was missing from the doc.
- [2026-09-19] docs/hermes-integration.md:17: context_engine listed as a
  hook in the architecture diagram; it is not a hook - plugin.yaml:28
  declares provides_context_engine, and the Hermes context engine registers
  only when APHRODITE_CONTEXT_ENGINE=1 (plugins/aphrodite/**init**.py:1206-1214);
  the per-turn catalog is injected via pre_llm_call by default.
- [2026-09-19] docs/hermes-integration.md:12,26: proxy launch attributed to
  on_session_start; proxies actually launch at plugin load via _start_proxy()
  with a health-probe skip (plugins/aphrodite/**init**.py:890,1216;
  APHRODITE_NO_AUTO_LAUNCH opt-out at 915).
- [2026-09-19] docs/hermes-integration.md:4,90: tree/Development links;
  default branch is Current (remotes/Source/HEAD -> Source/Current).
- [2026-09-19] docs/hermes-tool-output-schemas.md:5: links
  docs/ccr/content-types.md, which does not exist post-rewrite; taxonomy
  lives at docs/classification/content-types.md.
- [2026-09-19] docs/hermes-tool-output-schemas.md:5,22: tree/Development
  links; default branch is Current (remotes/Source/HEAD -> Source/Current).
- [2026-09-19] docs/hermes-tool-output-schemas.md:10,27: "22 classification
  types" stale; the shipped pipeline emits 30 types
  (docs/classification/content-types.md:118-151).
- [2026-09-19] docs/hermes-tool-output-schemas.md:29-52: taxonomy table uses
  playbook-only type names (search_files, search_results, tabular, commit,
  process_output, skill_view, write_file, browser_snapshot); shipped names
  are search/grep, table, gitlog/git, json/json_array, log
  (docs/classification/content-types.md:118-151).
- [2026-09-19] docs/hermes-tool-output-schemas.md:7: points into
  Maintain/hermes_tool_output_formats.json; Maintain/ is dev-side
  (Maintain/install.sh removed from the release line) - the 43-tool catalog
  is kept but the path is not public-facing.
- [2026-09-19] docs/hermes-tool-output-schemas.md:315-339: "New Classifier
  Types to Add" WIP section; log/write_file/browser_snapshot are shipped in
  1.4.6 (log produced by the proxy ladder,
  docs/classification/content-types.md:137).
- [2026-09-19] docs/hermes-tool-output-schemas.md:228: browser_console
  "Classify: log (new type)" - log is a shipped type produced by the proxy
  ladder (docs/classification/content-types.md:137).
