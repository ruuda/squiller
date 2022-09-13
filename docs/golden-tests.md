# Golden tests

Squiller has a suite of golden tests: inputs with known-good outputs. At the
time of writing they are used to test error reporting for incorrect inputs, but 
they should be extended to testing generated code. Golden tests are a good fit
for testing the parser and error reporting, because textual input is easier to
construct than manually constructing an AST in a unit test.

Golden tests are located in the `golden` directory, and can be executed with
`golden/run.py`. See also `run.py --help` for usage.

Test cases are files with a `.test` extension. The file consists of the test
input, which is fed to `sqiller` on stdin, then two blank lines, and then the
expected output. When the actual output does not match the expected output,
`run.py` prints a diff.
