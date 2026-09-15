#!/usr/bin/env python3
"""
verify_labels.py - verify bench/corpus fixture labels against the real
vendored headroom content detector.

For every fixture in bench/corpus/ this script:
  1. loads the REAL classifier the project uses - the vendored pure-python
     headroom content_detector (same import path as
     Maintain/scripts/bench/benchmark-eval.py::classify, which
     benchmark-report.py delegates to):
         vendor/headroom/headroom/transforms/content_detector.py
     imported via sys.path.insert(0, ...), exactly like the benchmark scripts.
  2. reads the fixture bytes, decodes UTF-8 (errors=replace - interior_nul.json
     deliberately carries raw NUL bytes), and runs detect_content_type().
  3. compares the detected label against the intended label from
     metadata.json and prints one row per fixture.

Defensive by design (repo convention):
  * If the detector is NOT importable (moved, deleted, or broken), the script
    does NOT crash - it prints a clear 'DETECTOR_UNAVAILABLE' note and falls
    back to the raw first-line heuristic classifier mirrored from
    benchmark-eval.py's fallback (same label vocabulary).
  * If metadata.json is missing, it degrades to detect-only mode (intended
    label shown as 'unknown').
  * Every per-fixture classify call is wrapped; a detector exception on one
    file is reported as 'detector_error', never a traceback.
  * Read-only: never writes, never commits.

Usage:
  python3 verify_labels.py          # table + summary, exit 0
  python3 verify_labels.py --quiet  # table only

Exit code is 0 on completion even when labels mismatch - this is a
MEASUREMENT tool, not a gate. Non-zero only if the corpus itself is missing.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent
# bench/corpus/verify_labels.py -> parents[0]=bench, [1]=Aphrodite (repo root)
REPO_ROOT = CORPUS_DIR.parents[1]
DETECTOR_DIR = REPO_ROOT / "vendor" / "headroom" / "headroom" / "transforms"
DETECTOR_FILE = DETECTOR_DIR / "content_detector.py"
METADATA_FILE = CORPUS_DIR / "metadata.json"

# Fallback discovery when metadata.json is absent: fixture-shaped files.
# gen_corpus.py is included (it is a corpus entry in metadata.json);
# verify_labels.py / README.md / metadata.json are never classified.
FIXTURE_GLOBS = ("*.txt", "*.json", "*.rs", "*.go", "*.py", "*.patch")
SELF_EXCLUDED = {"verify_labels.py", "README.md", "metadata.json"}

DETECTOR_UNAVAILABLE = "DETECTOR_UNAVAILABLE"


# ── Real classifier: vendored headroom content_detector ──────────────────
# Mirrors Maintain/scripts/bench/benchmark-eval.py::classify exactly:
#   DETECTOR_DIR.joinpath("content_detector.py").exists()
#   sys.path.insert(0, str(DETECTOR_DIR))
#   import content_detector
#   content_detector.detect_content_type(text).content_type.value
def load_detector():
    """Return the content_detector module, or None (with reason) if unusable."""
    if not DETECTOR_FILE.exists():
        return None, f"{DETECTOR_FILE} does not exist"
    try:
        sys.path.insert(0, str(DETECTOR_DIR))
        import content_detector  # type: ignore[import-not-found]

        # Sanity: the public entry point must exist and return an enum.
        probe = content_detector.detect_content_type("plain text probe")
        _ = probe.content_type.value
        return content_detector, None
    except Exception as e:  # noqa: BLE001 - any import/interface failure degrades
        return None, f"{type(e).__name__}: {e}"


# ── Raw heuristic fallback (benchmark-eval.py fallback classifier) ───────
def heuristic_classify(text: str) -> str:
    """First-line-pattern classifier with the same label vocabulary.

    Mirrored from Maintain/scripts/bench/benchmark-eval.py::classify's
    fallback branch so degraded output stays comparable to the real labels.
    """
    lines = text.splitlines() or [""]
    if text.lstrip().startswith(("{", "[")) and text.rstrip().endswith(("}", "]")):
        try:
            json.loads(text)
            return "json_array"
        except Exception:
            pass
    if re.match(r"^[^\s:]+:\d+:", lines[0]):
        return "search"
    if re.match(r"^(diff --git|--- a/|@@\s+-\d+,\d+\s+\+\d+,\d+\s+@@)", text):
        return "diff"
    if re.match(
        r"^\s*(def|class|import|from|async def|pub fn|fn |impl |use |#include|func |const |let |@|//|#!)",
        lines[0],
    ):
        return "source_code"
    if re.search(r"Compiling|error\[|warning:|Finished|test result:|running \d+ tests", text):
        return "build"
    if re.match(r"^<(!DOCTYPE|html)", text):
        return "html"
    if re.match(r"^\|.*\|$", lines[0]) or re.match(r"^[\w-]+,[\w-]+", lines[0]):
        return "tabular"
    return "text"


def load_intended() -> dict:
    """Intended labels from metadata.json; {} if absent/broken."""
    try:
        with open(METADATA_FILE, "r", encoding="utf-8") as fh:
            meta = json.load(fh)
        files = meta.get("files", {})
        return {name: entry.get("content_type", "unknown") for name, entry in files.items()}
    except Exception:  # noqa: BLE001
        return {}


def discover_fixtures(intended: dict) -> list:
    """metadata.json entries when present, else fixture-shaped glob."""
    if intended:
        return sorted(intended)
    found = sorted(
        p.name
        for p in CORPUS_DIR.iterdir()
        if p.is_file()
        and p.name not in SELF_EXCLUDED
        and p.suffix.lstrip(".") in {g.lstrip("*.") for g in FIXTURE_GLOBS}
    )
    return found


def classify_fixture(name: str, detector, heuristic: bool) -> tuple:
    """Detect one fixture. Never raises. Returns (detected, conf, note)."""
    path = CORPUS_DIR / name
    try:
        raw = path.read_bytes()
    except Exception as e:  # noqa: BLE001
        return "read_error", None, f"unreadable: {e}"
    try:
        text = raw.decode("utf-8", errors="replace")
    except Exception as e:  # noqa: BLE001
        return "decode_error", None, f"undecodable: {e}"
    if detector is not None:
        try:
            result = detector.detect_content_type(text)
            return result.content_type.value, getattr(result, "confidence", None), ""
        except Exception as e:  # noqa: BLE001
            # One pathological fixture must never take down the whole report.
            return "detector_error", None, f"detector raised {type(e).__name__}: {e}"
    try:
        return heuristic_classify(text), None, "heuristic"
    except Exception as e:  # noqa: BLE001
        return "heuristic_error", None, f"heuristic raised {type(e).__name__}: {e}"


def main() -> int:
    if not CORPUS_DIR.is_dir():
        print(f"error: corpus directory not found: {CORPUS_DIR}", file=sys.stderr)
        return 1

    quiet = "--quiet" in sys.argv[1:]

    detector, det_reason = load_detector()
    if detector is not None:
        print(f"# detector: vendored headroom content_detector ({DETECTOR_FILE}) - OK")
    else:
        print(f"# {DETECTOR_UNAVAILABLE}: {det_reason}")
        print("#   falling back to raw heuristic classifier (benchmark-eval.py fallback);")
        print("#   detected labels below are HEURISTIC, not the real classifier.")

    intended = load_intended()
    if not intended:
        print("# note: metadata.json missing/unreadable - intended labels unknown; detect-only mode")

    fixtures = discover_fixtures(intended)
    if not fixtures:
        print("# note: no fixtures found in corpus directory")
        return 0

    hdr = f"{'fixture':<24} {'intended':<16} {'detected':<16} status"
    print(hdr)
    print("-" * len(hdr))

    matched = mismatched = 0
    for name in fixtures:
        detected, conf, note = classify_fixture(name, detector, detector is None)
        want = intended.get(name, "unknown")
        ok = detected == want and detected not in ("read_error", "decode_error", "detector_error", "heuristic_error")
        if ok:
            matched += 1
            status = "OK"
        else:
            mismatched += 1
            status = "MISMATCH" if detected not in ("read_error", "decode_error", "detector_error", "heuristic_error") else "ERROR"
        conf_s = f" ({conf:.2f})" if conf is not None else ""
        extra = f" [{note}]" if note else ""
        print(f"{name:<24} {want:<16} {detected + conf_s:<16} {status}{extra}")

    print("-" * len(hdr))
    print(f"# summary: {matched} match, {mismatched} mismatch/error across {len(fixtures)} fixtures")
    if detector is None:
        print(f"# {DETECTOR_UNAVAILABLE} - results above are heuristic, not the real classifier")
    return 0


if __name__ == "__main__":
    sys.exit(main())