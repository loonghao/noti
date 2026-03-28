#!/usr/bin/env python3
"""Package OpenClaw skills into distributable zip archives.

Usage:
    python scripts/package_openclaw_skill.py

Output:
    dist/skills/noti-cli.zip
"""

import shutil
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SKILLS_DIR = REPO_ROOT / "skills"
DIST_DIR = REPO_ROOT / "dist" / "skills"


def package_skill(skill_dir: Path) -> Path:
    """Package a single skill directory into a zip archive."""
    if not skill_dir.is_dir():
        print(f"SKIP: {skill_dir} is not a directory", file=sys.stderr)
        return None

    skill_md = skill_dir / "SKILL.md"
    if not skill_md.exists():
        print(f"SKIP: {skill_dir.name} has no SKILL.md", file=sys.stderr)
        return None

    DIST_DIR.mkdir(parents=True, exist_ok=True)

    archive_base = DIST_DIR / skill_dir.name
    archive_path = shutil.make_archive(str(archive_base), "zip", str(skill_dir.parent), skill_dir.name)

    print(f"  ✓ {skill_dir.name} → {archive_path}")
    return Path(archive_path)


def main():
    print("Packaging OpenClaw skills…")
    print()

    if not SKILLS_DIR.exists():
        print("No skills/ directory found.", file=sys.stderr)
        sys.exit(1)

    archives = []
    for child in sorted(SKILLS_DIR.iterdir()):
        result = package_skill(child)
        if result:
            archives.append(result)

    print()
    if archives:
        print(f"Done — {len(archives)} skill(s) packaged in {DIST_DIR}")
    else:
        print("No skills found to package.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
