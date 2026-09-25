# Resolving a submodule merge conflict (parallel implementations)

Reference for SKILL.md ("Resolving a submodule merge conflict"). The rules live in
SKILL.md; full worked detail here. All commands are byte-stable.

Auto-committers on both sides often implement the SAME feature independently - the merge
shows "both modified" on the same files:

- Scope both sides first - one is usually a SUPERSET (extra commands/tests). Take the
  superset's implementation; keep the other side's text only where it documents something
  the superset misses (e.g. accurate legacy-behavior notes).
- Dedupe everything both sides added twice: guards, tests, env-var entries, identical
  gitlink bumps. Check `plugin.yaml`/manifest-style files too - both sides append the same
  line.
- Scripted marker-block replacement (`<<<<<<<` ... `>>>>>>>`) must end the resolution text
  with the FILE's line terminator: a missing trailing CRLF glues the next code line onto
  the resolution, often INTO a comment (py_compile still passes; the NameError surfaces
  only at import/test time). Use a script that asserts exactly one occurrence per block
  and re-reads file bytes (mixed CRLF/LF lines are normal after a merge).
- The fuzzy patch tool re-indents multi-line blocks on CRLF/tab-indented files; after two
  mangled attempts on the same region, switch to an exact-byte replacement script.
