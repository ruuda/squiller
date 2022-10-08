{
  description = "Squiller";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: 
    let
      supportedSystems = [ "x86_64-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];
      # Ridiculous boilerplate required to make flakes somewhat usable.
      forEachSystem = f:
        builtins.zipAttrsWith
          (name: values: builtins.foldl' (x: y: x // y) {} values)
          (map
            (k: builtins.mapAttrs (name: value: { "${k}" = value; }) (f k))
            supportedSystems
          );
    in
      forEachSystem (system:
        let
          name = "squiller";
          version = "0.1.0";
          pkgs = import nixpkgs { inherit system; };
        in
          rec {
            devShells.default = pkgs.mkShell {
              nativeBuildInputs = [
                (with pkgs.python3.pkgs; toPythonApplication pygments)
                pkgs.mkdocs
                pkgs.python3
                pkgs.rustup
                pkgs.sqlite
              ];
            };

            checks = rec {
              debugBuild = packages.default.overrideAttrs (old: {
                name = "check-test";
                buildType = "debug";
                # We don't want to set the version number in this case,
                # because it would interfere with the output of the goldens.
                patchPhase = "";
              });

              golden = pkgs.runCommand
                "check-golden"
                { buildInputs = [ pkgs.python3 ]; }
                ''
                cd ${pkgs.lib.sourceFilesBySuffices ./. [".py" ".test"]}
                SQUILLER_BIN=${debugBuild}/bin/squiller python3 golden/run.py
                touch $out
                '';

              fmt = pkgs.runCommand
                "check-fmt"
                { buildInputs = [ pkgs.cargo pkgs.rustfmt ]; }
                ''
                cargo fmt --manifest-path ${./.}/Cargo.toml -- --check
                touch $out
                '';

              # TODO: Try to get Clippy to work with `buildRustPackage` ...
              # Maybe I should switch to Naersk after all.
            };

            packages = {
              default = pkgs.rustPlatform.buildRustPackage rec {
                inherit name version;
                src = pkgs.lib.sourceFilesBySuffices ./. [
                  ".rs"
                  ".sql"
                  "Cargo.lock"
                  "Cargo.toml"
                ];
                cargoLock.lockFile = ./Cargo.lock;
                nativeBuildInputs = [ pkgs.sqlite ];
                rev = if self ? rev then ''Some("${self.rev}")'' else "None";
                versionSrc =
                  ''
                  pub const VERSION: &'static str = "${version}";
                  pub const REV: Option<&'static str> = ${rev};
                  '';
                patchPhase = ''echo "$versionSrc" > src/version.rs'';
              };
            };
          }
      );
}
