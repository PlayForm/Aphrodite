#!/usr/bin/env python3
"""
Verify Git repository cleanliness after large-file cleanup.
Checks for any files exceeding size thresholds and confirms LFS status.
Works for the Aphrodite monorepo root or any submodule (plugins/aphrodite,
vendor/headroom, vendor/rtk).
"""

import os
import subprocess
import sys
from pathlib import Path

SIZE_THRESHOLD_MB = 50  # Flag files larger than this


def run_cmd(cmd, cwd=None):
    """Run shell command, return (stdout, exit_code)."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cwd)
    return result.stdout.strip(), result.returncode


def check_for_large_files(repo_path, threshold_mb=50):
    """Find files exceeding threshold (excluding .git)."""
    threshold_bytes = threshold_mb * 1024 * 1024
    large_files = []

    for root, dirs, files in os.walk(repo_path):
        # Skip .git directories
        if ".git" in root:
            continue
        for f in files:
            fp = Path(root) / f
            try:
                size = fp.stat().st_size
                if size > threshold_bytes:
                    large_files.append((str(fp), size))
            except OSError:
                continue

    return sorted(large_files, key=lambda x: x[1], reverse=True)


def verify_clean_repo(repo_path=None):
    """Main verification routine."""
    repo_path = Path(repo_path or os.getcwd()).resolve()
    print(f"Verifying repo: {repo_path}")

    # 1. Check we're in a git repo
    stdout, code = run_cmd("git rev-parse --is-inside-work-tree", cwd=repo_path)
    if code != 0 or stdout != "true":
        print("❌ Not inside a Git working tree")
        return False

    print("✅ Git working tree confirmed")

    # 2. Check remote URL format (GitHub SSH)
    stdout, code = run_cmd("git remote get-url origin", cwd=repo_path)
    if code == 0:
        if stdout.startswith("git@github.com:"):
            print(f"✅ Remote URL format correct: {stdout}")
        else:
            print(f"⚠️  Remote URL may be non-standard: {stdout}")
            print("   Recommended: git@github.com:user/repo.git")
    else:
        print("⚠️  No 'origin' remote configured")

    # 3. Check for large files in working tree
    large = check_for_large_files(repo_path, SIZE_THRESHOLD_MB)
    if large:
        print(f"\n❌ Found {len(large)} file(s) > {SIZE_THRESHOLD_MB}MB:")
        for path, size in large[:10]:  # show top 10
            mb = size / (1024 * 1024)
            print(f"   {mb:.1f} MB - {path}")
        return False
    else:
        print(f"✅ No files > {SIZE_THRESHOLD_MB}MB in working tree")

    # 4. Verify .gitignore exists and is non-empty
    gitignore = repo_path / ".gitignore"
    if gitignore.exists():
        lines = gitignore.read_text().splitlines()
        print(f"✅ .gitignore exists ({len(lines)} rules)")
    else:
        print("⚠️  No .gitignore found")

    # 5. LFS status (if LFS is installed)
    stdout, code = run_cmd("git lfs version", cwd=repo_path)
    if code == 0:
        print("✅ Git LFS installed")
        stdout, code = run_cmd("git lfs status", cwd=repo_path)
        if code == 0 and stdout.strip():
            print("   LFS-tracked patterns active:")
            for line in stdout.splitlines():
                print(f"     {line}")
        elif code == 0:
            print("   No LFS-tracked files in working tree")
    else:
        print("ℹ️  Git LFS not installed (optional)")

    # 6. Show repo size
    stdout, code = run_cmd("du -sh .", cwd=repo_path)
    if code == 0:
        print(f"Repository size: {stdout}")

    print("\n✅ Repository cleanliness verification passed")
    return True


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Verify Git repo cleanliness")
    parser.add_argument("path", nargs="?", default=".", help="Path to repository")
    args = parser.parse_args()

    success = verify_clean_repo(args.path)
    sys.exit(0 if success else 1)
