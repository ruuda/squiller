#!/usr/bin/env python3

"""
Update all generated code in the `examples` directory.

Usage:

    tools/update_examples.py
"""

import os
import os.path
import subprocess


def generate_example(in_fname: str, target: str, extension: str) -> str:
    base = os.path.splitext(in_fname)[0]
    out_fname = base + "_" + target.replace("-", "_") + extension

    cmd = ["target/debug/squiller", f"--target={target}", in_fname]
    result = subprocess.run(cmd, stdout=subprocess.PIPE, check=True)

    with open(out_fname, "wb") as f:
        f.write(result.stdout)

    return out_fname


def main() -> None:
    files_rs = []
    files_py = []

    for fname in os.listdir("examples"):
        if not fname.endswith(".sql"):
            continue
        in_fname = os.path.join("examples", fname)
        files_rs.append(generate_example(in_fname, "rust-sqlite", ".rs"))
        files_py.append(generate_example(in_fname, "python-psycopg2", ".py"))
        files_py.append(generate_example(in_fname, "python-sqlite", ".py"))

    subprocess.run(["black", *files_py])
    subprocess.run(["rustfmt", *files_rs])


if __name__ == "__main__":
    main()
