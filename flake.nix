{
  description = "A basic flake with a shell";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];

        pkgs = import nixpkgs {inherit system overlays;};

        rustNightly =
          pkgs.rust-bin.selectLatestNightlyWith
          (toolchain:
            toolchain.default.override {
              extensions = [
                "rust-src"
                "rustfmt"
                "clippy"
                "cargo"
                "rust-analyzer"
              ];
            });

        manifest = pkgs.lib.importTOML ./Cargo.toml;

        default = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.package.name;
          version = manifest.package.version;

          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          release = true;

          # testing require external database
          doCheck = false;

          env = {
            SQLX_OFFLINE = true;
          };
        };
      in {
        defaultPackage = default;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.openssl
            pkgs.pkg-config
            rustNightly
          ];
          packages = with pkgs; [
            bashInteractive

            # ci
            act

            # db
            postgresql

            sqlx-cli
            cargo-udeps
            jq
            sqruff

            # digital ocean
            doctl
            # faster build on deployment
            cargo-chef
          ];
        };
      }
    );
}
