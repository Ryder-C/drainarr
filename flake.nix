{
  description = "drainarr - drain your *arr library to a disk-usage target";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }: let
    perSystem = flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      packages = rec {
        drainarr = pkgs.rustPlatform.buildRustPackage {
          pname = "drainarr";
          version = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type: let
              base = baseNameOf path;
            in
              !(builtins.elem base ["target" "result" ".dev-env" ".git" ".jj"]);
          };

          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "Drain your *arr library to a configured disk-usage target";
            homepage = "https://github.com/Ryder-C/drainarr";
            license = licenses.mit;
            mainProgram = "drainarr";
            platforms = platforms.linux;
          };
        };

        default = drainarr;
      };

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
          rustfmt
          clippy
          rust-analyzer
          pkg-config
        ];

        RUST_LOG = "info,drainarr=debug";
      };

      formatter = pkgs.alejandra;
    });
  in
    perSystem
    // {
      nixosModules = rec {
        drainarr = import ./nix/module.nix self;
        default = drainarr;
      };

      overlays.default = final: prev: {
        drainarr = self.packages.${prev.stdenv.hostPlatform.system}.drainarr;
      };
    };
}
