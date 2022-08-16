# Building

Querybinder is written in Rust and builds with Cargo:

    cargo build --release
    target/release/querybinder

## Development

Run the unit tests:

    cargo test

Run the [golden tests](golden-tests.md):

    golden/run.py

Run one of the fuzz tests (in this case `typecheck`):

    cargo +nightly-2022-06-25 fuzz run typecheck

Build the documentation or view it locally:

    mkdocs build
    mkdocs serve

## Nix

You can enter a development environment with pinned build tools on the PATH with
[Nix 2.10][nix]. You need to run Nix either with:

    --extra-experimental-features nix-command
    --extra-experimental-features flakes

or you can add these settings to your `~/.config/nix/nix.conf`. Then enter a
development shell:

    nix develop --command $SHELL

The Nix flake can also be used to build the application, although this is not
the default workflow and may break from time to time:

    nix build
    result/bin/querybinder --help

[nix]: https://nixos.org/download.html
