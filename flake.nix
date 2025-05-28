{
  description = "A basic flake with a shell";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
  };

  outputs = {
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};

        sqlx-cli = pkgs.rustPlatform.buildRustPackage rec {
          pname = "sqlx-cli";
          version = "0.8.6";
          src = pkgs.fetchFromGitHub {
            owner = "launchbadge";
            repo = "sqlx";
            rev = "refs/tags/v${version}";
            hash = "sha256-Trnyrc17KWhX8QizKyBvXhTM7HHEqtywWgNqvQNMOAY=";
          };
          buildAndTestSubdir = "sqlx-cli";
          cargoHash = "sha256-FxvzCe+dRfMUcPWA4lp4L6FJaSpMiXTqEyhzk+Dv1B8=";
          buildNoDefaultFeatures = true;
          buildFeatures = [
            "postgres"
          ];
          doCheck = false;
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs;
            [
              bashInteractive

              # ci
              act

              # rust dev
              cargo
              clippy
              rustc
              rustfmt
              rust-analyzer

              # db
              postgresql
            ]
            ++ [sqlx-cli];
        };
      }
    );
}
