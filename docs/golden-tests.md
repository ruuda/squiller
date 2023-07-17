# Golden tests

Squiller has a suite of golden tests: inputs with known-good outputs. These test
error reporting of incorrect inputs, as well as code generation for correct
inputs. Golden tests are a good fit for testing the parser and error reporting,
because textual input is easier to construct than manually constructing an AST
in a unit test.

Golden tests are located in the `golden` directory, and can be executed with
`golden/run.py`. See also `run.py --help` for usage. Inside the `golden`
directory is a subdirectory per target, and an additional directory `error`
for error reporting tests.

Test cases are files with a `.test` extension. The file consists of the test
input, which is fed to `sqiller` on stdin, then two blank lines, and then the
expected output. When the actual output does not match the expected output,
`run.py` prints a diff.
