{
  description = "Squiller";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: 
    let
      name = "squiller";
      version = "0.1.0";
      supportedSystems = [ "x86_64-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      forAllNixpkgs = f: forAllSystems (system: f (import nixpkgs { inherit system; }));
    in
      {
        devShells = forAllNixpkgs (pkgs: {
          default = pkgs.mkShell {
            nativeBuildInputs = [
              (with pkgs.python3.pkgs; toPythonApplication pygments)
              pkgs.mkdocs
              pkgs.python3
              pkgs.rustup
              pkgs.sqlite
            ];
          };
        });

        packages = forAllNixpkgs (pkgs: {
          default = pkgs.rustPlatform.buildRustPackage {
            inherit name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.sqlite ];
            versionSrc =
              ''
              pub const VERSION: &'static str = "${version}";
              pub const REV: Option<&'static str> = Some("${self.rev}");
              '';

            patchPhase = ''echo "$versionSrc" > src/version.rs'';
          };
        });
      };
}
