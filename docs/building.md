# Building

Squiller is written in Rust and builds with Cargo:

    cargo build --release
    target/release/squiller

## Nix

The repository comes with a Nix-based development environment that puts pinned
versions of the necessary build tools on the PATH. Alternatively, you can
source the build tools manually. To enter a development environment with
[Nix 2.10][nix], you need to run Nix either with:

    --extra-experimental-features nix-command
    --extra-experimental-features flakes

or you can add these settings to your `~/.config/nix/nix.conf`. Then enter a
development shell:

    nix develop --command $SHELL

The Nix flake can also be used to build the application. This is not recommended
for development because you lose incremental compilation, but flakes can be a
useful way of integrating Squiller into the build pipeline of a different
project. To build the flake:

    nix build
    result/bin/squiller --help

[nix]: https://nixos.org/download.html

## Development

Run the unit tests:

    cargo test

Run the [golden tests](golden-tests.md):

    golden/run.py

Run one of the fuzz tests (in this case `typecheck`):

    cargo +nightly-2022-06-25 fuzz run typecheck -- -dict=fuzz/dictionary.txt

Build the documentation or view it locally:

    mkdocs build
    mkdocs serve
