{
  description = "Minimal Valence co-processor dev environment";

  nixConfig = {
    extra-experimental-features = "nix-command flakes";
    allow-import-from-derivation = true; # Keep for rust-overlay if it needs it
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = { config, pkgs, system, ... }:
      let
        overlays = [ (import inputs.rust-overlay) ];
        pkgsWithOverlays = import inputs.nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgsWithOverlays.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.pkg-config
            pkgs.openssl
            pkgs.clang
            pkgs.llvmPackages.llvm
          ];
        };
      };
    };
}