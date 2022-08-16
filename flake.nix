{
  description = "Querybinder";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: 
    let
      name = "querybinder";
      version = builtins.substring 0 8 self.lastModifiedDate;
      supportedSystems = [ "x86_64-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      forAllNixpkgs = f: forAllSystems (system: f (import nixpkgs { inherit system; }));
    in
      {
        devShells = forAllNixpkgs (pkgs: {
          default = pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.mkdocs
              pkgs.rustup
            ];
          };
        });

        packages = forAllNixpkgs (pkgs: {
          default = pkgs.rustPlatform.buildRustPackage {
            inherit name version;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };
        });
      };
}