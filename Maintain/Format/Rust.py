#!/usr/bin/env python3
"""
Rust.py - Insert blank lines after ;  }  ,  and { in Rust source files.

Rules (state-machine, line-by-line):

  After ;  }
  ----------
  - Insert a blank line when the next line is non-blank AND does not start
    with a closing delimiter (}  )  ]  ,  ;) or a chain call (.method()).
  - Guards: skip lines inside /* */ comments, inside `use { ... }` import
    blocks, and while ParenDepth > 0 (inside macro calls like Log!(),
    format!(), or function arguments).

  After ,
  -------
  - Same as above, but the paren guard is relaxed: commas inside direct
    macro arguments DO get a blank line after them.
  - "Direct macro args" means ParenDepth > 0 AND the net brace depth since
    the enclosing paren opened is 0 - i.e., we have not entered a struct
    literal or closure body inside the macro call.
  - Commas inside struct literals or closure bodies within a macro call
    (e.g. json!({ "k": V, }) or .map_err(|E| { Field: x, })) are left alone
    because RelativeBraceDepth > 0.

  After {
  -------
  - Insert a blank line only for top-level block openings - impl, enum,
    struct, mod (brace depth becomes <= OpenBraceMaxDepth after the line).
  - Deeply nested { (if/for/match arms) are intentionally left alone.

  General
  -------
  - Strings ("..."), char literals ('.'), raw strings (r#"..."#), and
    block comments (/* ... */) are scanned so delimiters inside them never
    corrupt depth tracking or trigger insertion.
  - Never insert a second consecutive blank line (next line already blank -> skip).

Usage:
    # Dry-run on target file:
    python Maintain/Format/Rust.py --DryRun crates/aphrodite/src/lib.rs

    # Apply to one file:
    python Maintain/Format/Rust.py crates/aphrodite/src/lib.rs

    # Recursively apply to all .rs files:
    python Maintain/Format/Rust.py --All

    # Dry-run everything:
    python Maintain/Format/Rust.py --DryRun --All

    # Allow blank after { up to brace depth 2 (fn bodies too):
    python Maintain/Format/Rust.py --OpenBraceDepth 2 --All
"""

import re
import sys
from pathlib import Path
from typing import TypedDict


MatchClosingToken = re.compile(r"^\s*[})\];,]")
MatchChainContinue = re.compile(r"^\s*\.")
MatchCommentLine = re.compile(r"^\s*(//|/\*|\*)")

# Directory components that are never formatted. Mirrors the exclusion sets in
# rustfmt.toml `ignore`, Maintain/Format.sh find paths, and .prettierignore.
#
# `vendor` and `plugins` are separate git submodules (PlayForm/Headroom,
# PlayForm/rtk, PlayForm/Aphrodite-Hermes) with their own formatting - never
# reformat their style from here. `profiles` is gitignored, generated at
# `aphrodite setup` time.
Exclude = frozenset(
    {
        "vendor",
        "plugins",
        "profiles",
        ".hermes",
        "target",
        "node_modules",
        ".git",
        ".fingerprint",
        "incremental",
        "deps",
        "Generated",
        ".generated",
        "gen",
        "bin",
    }
)


class ScanResult(TypedDict):
    ParenDelta: int
    BraceDelta: int
    LastChar: str | None
    EndsInBlock: bool
    EndsInStr: bool


def ScanLine(Line: str, MidBlockComment: bool = False) -> ScanResult:
    """
    Scan a single Rust source line, skipping string/char/raw-string/content
    and block comments. Return a dict with depth deltas and metadata.

    Parameters
    ----------
    Line             : the source line to scan.
    MidBlockComment  : True if the line starts inside an open /* */ comment.

    Returns
    -------
    ParenDelta   : int  - net parenthesis depth change
    BraceDelta   : int  - net brace depth change
    LastChar     : str|None - last non-whitespace code character on the line
    EndsInBlock  : bool - the line ends with an unclosed /*
    EndsInStr    : bool - the line ends mid-string / mid-char / mid-raw-str
    """
    ParenDelta = 0
    BraceDelta = 0
    LastChar = None
    Position = 0
    Length = len(Line)

    InBlock = MidBlockComment
    InDouble = False
    InChar = False
    InRaw = False
    RawHashCount = 0

    while Position < Length:
        Character = Line[Position]

        if InDouble:
            if Character == "\\":
                Position += 2
                continue
            if Character == '"':
                InDouble = False
            Position += 1
            continue

        if InChar:
            if Character == "\\":
                Position += 2
                continue
            if Character == "'":
                InChar = False
            Position += 1
            continue

        if InRaw:
            if Character == '"':
                Hashes = 0
                NextHash = Position + 1
                while NextHash < Length and Line[NextHash] == "#":
                    Hashes += 1
                    NextHash += 1
                if Hashes >= RawHashCount:
                    InRaw = False
                    Position = Position + 1 + RawHashCount
                else:
                    Position += 1
                continue
            Position += 1
            continue

        if InBlock:
            if Character == "*" and Position + 1 < Length and Line[Position + 1] == "/":
                InBlock = False
                Position += 2
                continue
            Position += 1
            continue

        if Character in ("r", "b") and not InDouble and not InChar:
            Start = Position
            if Character == "b" and Position + 1 < Length and Line[Position + 1] == "r":
                Start += 1
            Next = Start + 1
            if Next < Length and Line[Next] == "r":
                Next += 1
            Count = 0
            while Next < Length and Line[Next] == "#":
                Count += 1
                Next += 1
            if Next < Length and Line[Next] == '"':
                InRaw = True
                RawHashCount = Count
                Position = Next + 1
                continue

        if Character == '"':
            InDouble = True
            Position += 1
            continue

        if Character == "'":
            PeekPosition = Position + 1
            if PeekPosition < Length:
                if Line[PeekPosition] == "\\":
                    PeekPosition += 2
                    if PeekPosition < Length and Line[PeekPosition] == "'":
                        InChar = True
                        Position += 1
                        continue
                elif Line[PeekPosition] != "'":
                    NextQuote = Line.find("'", PeekPosition + 1)
                    if NextQuote > 0 and NextQuote - PeekPosition <= 2:
                        InChar = True
                        Position += 1
                        continue
            LastChar = Character
            Position += 1
            continue

        if Character == "/" and Position + 1 < Length and Line[Position + 1] == "/":
            break

        if Character == "/" and Position + 1 < Length and Line[Position + 1] == "*":
            InBlock = True
            Position += 2
            continue

        if Character == "(":
            ParenDelta += 1
        elif Character == ")":
            ParenDelta -= 1
        elif Character == "{":
            BraceDelta += 1
        elif Character == "}":
            BraceDelta -= 1

        if Character not in (" ", "\t", "\r", "\n"):
            LastChar = Character

        Position += 1

    return {
        "ParenDelta": ParenDelta,
        "BraceDelta": BraceDelta,
        "LastChar": LastChar,
        "EndsInBlock": InBlock,
        "EndsInStr": InDouble or InChar or InRaw,
    }


def Transform(Source: str, OpenBraceMaxDepth: int = 1) -> str:
    LineList = Source.split("\n")
    Output: list[str] = []

    BlockCommentOpen = False
    ParenDepth = 0
    BraceDepth = 0
    BraceDepthImport = 0
    InUseBlock = False

    ParenOpenBraceStack: list[int] = []

    for Index, Line in enumerate(LineList):
        Scan = ScanLine(Line, MidBlockComment=BlockCommentOpen)
        BlockCommentOpen = Scan["EndsInBlock"]

        if not BlockCommentOpen and not Scan["EndsInStr"]:
            if re.match(r"\s*use\s+", Line):
                InUseBlock = True
            if InUseBlock:
                BraceDepthImport += Line.count("{") - Line.count("}")
                if BraceDepthImport <= 0:
                    InUseBlock = False
                    BraceDepthImport = 0

            CloseCount = max(0, -Scan["ParenDelta"])
            for _ in range(CloseCount):
                if ParenOpenBraceStack:
                    ParenOpenBraceStack.pop()
            for _ in range(max(0, Scan["ParenDelta"])):
                ParenOpenBraceStack.append(BraceDepth)

            ParenDepth = max(0, ParenDepth + Scan["ParenDelta"])
            BraceDepth = max(0, BraceDepth + Scan["BraceDelta"])

        Output.append(Line)

        InComment = BlockCommentOpen or bool(MatchCommentLine.match(Line))

        if InComment or InUseBlock or Scan["EndsInStr"]:
            continue

        Stripped = Line.rstrip()
        if not Stripped:
            continue

        Last = Scan["LastChar"]
        if Last is None:
            continue

        if Index + 1 >= len(LineList):
            continue

        NextLine = LineList[Index + 1]
        NextBlank = NextLine.strip() == ""
        NextClosing = bool(MatchClosingToken.match(NextLine))
        NextChain = bool(MatchChainContinue.match(NextLine))

        if NextBlank or NextClosing or NextChain:
            continue

        if Last in (";", "}", ")"):
            if ParenDepth == 0:
                Output.append("")
            elif ParenOpenBraceStack:
                RelativeBraceDepth = BraceDepth - ParenOpenBraceStack[-1]
                if RelativeBraceDepth > 0:
                    Output.append("")

        elif Last == ",":
            if ParenDepth == 0:
                Output.append("")
            elif ParenOpenBraceStack:
                RelativeBraceDepth = BraceDepth - ParenOpenBraceStack[-1]
                if RelativeBraceDepth == 0:
                    Output.append("")

        elif Last == "{" and ParenDepth == 0:
            if BraceDepth <= OpenBraceMaxDepth:
                Output.append("")

    return "\n".join(Output)


# ── CLI ───────────────────────────────────────────────────────────────────────


def ProcessFile(FilePath: Path, DryRun: bool, OpenBraceMaxDepth: int) -> bool:
    try:
        Text = FilePath.read_text(encoding="utf-8")
    except Exception as Error:
        print(f"  ERROR reading {FilePath}: {Error}", file=sys.stderr)
        return False

    NewText = Transform(Text, OpenBraceMaxDepth=OpenBraceMaxDepth)
    if NewText == Text:
        return False

    if DryRun:
        print(f"[DRY RUN] Would modify: {FilePath}")
    else:
        FilePath.write_text(NewText, encoding="utf-8")
        print(f"Modified: {FilePath}")
    return True


def Main() -> None:
    import argparse

    Parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    Parser.add_argument(
        "Files",
        nargs="*",
        help=".rs file paths (shell globs ok if quoted)",
    )
    Parser.add_argument(
        "--All",
        action="store_true",
        help="Recursively process every .rs file under the current directory",
    )
    Parser.add_argument(
        "--DryRun",
        action="store_true",
        help="Show what would change without writing anything",
    )

    def ParseDepth(Value: str) -> int:
        if Value.lower() in ("all", "inf", "infinite"):
            return 2**31
        try:
            return int(Value)
        except ValueError:
            raise argparse.ArgumentTypeError(
                f"Expected an integer or 'All', got: {Value!r}"
            )

    Parser.add_argument(
        "--OpenBraceDepth",
        type=ParseDepth,
        default=1,
        metavar="N|All",
        help=(
            "Insert blank after { only when brace depth after the line is <= N. "
            "Default 1 (impl/enum/struct/mod). Use 2 to also cover fn bodies. "
            "Pass All (or Inf) to apply at every nesting depth."
        ),
    )
    Args = Parser.parse_args()

    Target: list[Path] = []

    if Args.All:
        Target = sorted(
            File
            for File in Path(".").rglob("*.rs")
            if not any(Part in Exclude for Part in File.parts)
        )
    else:
        for Pattern in Args.Files:
            Expanded = sorted(Path(".").glob(Pattern))
            if Expanded:
                Target.extend(Expanded)
            else:
                Candidate = Path(Pattern)
                if Candidate.exists():
                    Target.append(Candidate)
                else:
                    print(f"Warning: no match for '{Pattern}'", file=sys.stderr)

    if not Target:
        Parser.print_help()
        sys.exit(1)

    Changed = sum(
        ProcessFile(File, Args.DryRun, Args.OpenBraceDepth) for File in Target
    )
    Total = len(Target)
    Verb = "would change" if Args.DryRun else "changed"
    print(f"\nDone - {Verb} {Changed}/{Total} file(s).")


if __name__ == "__main__":
    Main()
