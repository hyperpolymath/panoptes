# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

{
  description = "Panoptes - Local AI File Scanner & Renamer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ];
      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          packages = with pkgs; [
            # Build tools
            just
            cargo-audit
            cargo-deny
            cargo-outdated
            cargo-tarpaulin

            # Container tools
            podman
            podman-compose

            # Documentation
            lychee  # Link checker

            # Nickel configuration
            nickel

            # Oil shell
            oil
          ];

          shellHook = ''
            echo "Panoptes Development Environment"
            echo "================================="
            echo ""
            echo "Available commands:"
            echo "  just --list    # Show all tasks"
            echo "  just dev       # Full dev setup"
            echo "  just validate  # RSR compliance check"
            echo ""
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };

        # Package definition
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "panoptes";
          version = "1.0.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          meta = with pkgs.lib; {
            description = "Local AI-powered file scanner and renamer";
            homepage = "https://gitlab.com/hyperpolymath/panoptes";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.unix;
          };
        };

        # Checks
        checks = {
          # Run clippy
          clippy = pkgs.runCommand "clippy" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${self}
            cargo clippy -- -D warnings
            touch $out
          '';

          # Run tests
          test = pkgs.runCommand "test" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${self}
            cargo test
            touch $out
          '';

          # Format check
          fmt = pkgs.runCommand "fmt" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${self}
            cargo fmt -- --check
            touch $out
          '';
        };

        # Apps
        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };
      }
    );
}
