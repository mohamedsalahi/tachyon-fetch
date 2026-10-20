#!/usr/bin/env python3
"""Emit the cargo eclass CRATES list from Cargo.lock, or check an ebuild."""
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOCK = ROOT / "Cargo.lock"


def crates() -> list[str]:
    with LOCK.open("rb") as f:
        data = tomllib.load(f)
    return sorted(
        f"{p['name']}-{p['version']}"
        for p in data["package"]
        if p.get("source", "").startswith("registry+")
    )


def block(path: Path) -> list[str]:
    text = path.read_text()
    match = re.search(r'CRATES="(.*?)"', text, re.S)
    if not match:
        sys.exit(f"{path}: CRATES block not found")
    return match.group(1).split()


def main() -> None:
    expected = crates()
    if "--check" in sys.argv:
        path = Path(sys.argv[-1])
        actual = block(path)
        if sorted(actual) != expected:
            missing = sorted(set(expected) - set(actual))
            extra = sorted(set(actual) - set(expected))
            sys.exit(f"{path}: CRATES mismatch; missing={missing} extra={extra}")
        print(f"{path}: {len(expected)} crates ok")
        return
    print("\n".join(expected))


if __name__ == "__main__":
    main()
