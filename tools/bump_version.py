#!/usr/bin/env python3

"""
Bump the version number everywhere in the repository.

Usage:

    tools/bump_version.py <old-version> <new-version>
"""

from typing import List

import sys
import subprocess


def replace_lines(fname: str, needle: str, replacement: str) -> None:
    lines: List[str] = []
    with open(fname, "r", encoding="utf-8") as f:
        for line in f:
            if line == needle:
                lines.append(replacement)
            else:
                lines.append(line)

    with open(fname, "w", encoding="utf-8") as f:
        for line in lines:
            f.write(line)


def update_files(old_version: str, new_version: str) -> None:
    replace_lines(
        "Cargo.toml",
        needle=f'version = "{old_version}"\n',
        replacement=f'version = "{new_version}"\n',
    )
    replace_lines(
        "flake.nix",
        needle=f'          version = "{old_version}";\n',
        replacement=f'          version = "{new_version}";\n',
    )
    replace_lines(
        "src/version.rs",
        needle=f'pub const VERSION: &str = "{old_version}-dev";\n',
        replacement=f'pub const VERSION: &str = "{new_version}-dev";\n',
    )


def rebuild_generated_files() -> None:
    # Generate the binary needed for the golden tests.
    subprocess.run(["cargo", "build"], check=True)

    # Update goldens expected output, because those include a version number.
    subprocess.run(["golden/run.py", "--rewrite-output"], check=False)

    subprocess.run(["tools/update_examples.py"], check=True)

    # Run the fuzzer very briefly to make it update its Cargo.lock.
    subprocess.run(
        [
            "cargo",
            "+nightly-2022-06-25",
            "fuzz",
            "run",
            "generate",
            "--",
            "-dict=fuzz/dictionary.txt",
            "-max_len=32",
            "-runs=100",
        ]
    )


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__.strip())
        sys.exit(1)

    update_files(sys.argv[1], sys.argv[2])
    rebuild_generated_files()
