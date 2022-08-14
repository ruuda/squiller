#!/usr/bin/env python3

"""
A test runner for running the golden tests.

The runner takes golden input files, splits them into inputs and expectations,
and then prints whether they match.

Output is TAP (Test Anything Protocol) compliant, so you can run all of the
golden tests with Prove while using this script as interpreter (with --exec).

Standalone usage:
  golden/run.py file.t

Interpreter usage:
  prove --exec golden/run.py golden
"""

import difflib
import os
import re
import subprocess
import sys

from typing import Iterable, List, Iterator


STRIP_ESCAPES = re.compile("\x1b[^m]+m")


def main(fname: str) -> None:
    input_lines: List[str] = []
    golden_lines: List[str] = []

    with open(fname, "r", encoding="utf-8") as f:
        consecutive_blank = 0
        target = input_lines
        for line in f:
            target.append(line)

            if line == "\n":
                consecutive_blank += 1
            else:
                consecutive_blank = 0

            if consecutive_blank >= 2:
                target = golden_lines

    # Print the number of tests we are going to run,
    # in accordance with the TAP v12 protocol.
    print(f'1..1', flush=True)

    #Run with RUST_BACKTRACE=1 so we get a backtrace if the process panics.
    os.putenv('RUST_BACKTRACE', "1")

    result = subprocess.run(
        ['target/debug/querybinder', "--target=debug", "-"],
        input="".join(input_lines),
        capture_output=True,
        encoding="utf-8",
    )
    output_lines = [
        # Strip ANSI escape codes from the output.
        STRIP_ESCAPES.sub("", line)
        for line in result.stdout.splitlines() + result.stderr.splitlines()
    ]

    is_good = True
    red = "\x1b[31m";
    green = "\x1b[32m";
    reset = "\x1b[0m";

    for diff_line in difflib.unified_diff(
        a=output_lines,
        b=[line[:-1] for line in golden_lines],
        fromfile="actual",
        tofile="golden",
        lineterm="",
    ):
        is_good = False
        if diff_line.startswith("-"):
            print(red + diff_line + reset)
        elif diff_line.startswith("+"):
            print(green + diff_line + reset)
        else:
            print(diff_line)

    if is_good:
        print('ok 1', flush=True)
    else:
        fname_actual = fname + ".actual"
        print(f"not ok 1 - Output written to {fname_actual}", flush=True)

        with open(fname_actual, "w", encoding="utf-8") as f:
            for line in input_lines:
                f.write(line)
            for line in output_lines:
                f.write(line)
                f.write("\n")


if __name__ == '__main__':
    fname = None

    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(1)

    main(sys.argv[1])
