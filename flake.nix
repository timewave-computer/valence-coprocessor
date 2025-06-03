{
  description = "Valence co-processor: Zero-Knowledge Virtual Machine service for cross-chain applications";

  nixConfig = {
    extra-experimental-features = "nix-command flakes";
    allow-import-from-derivation = true;
    extra-substituters = "https://coffeetables.cachix.org";
    extra-trusted-public-keys = ''
      coffeetables.cachix.org-1:BCQXDtLGFVo/rTG/J4omlyP/jbtNtsZIKHBMTjAWt8g=
    '';
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell.url = "github:numtide/devshell";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-parts, devshell, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } ({moduleWithSystem, ...}: {
      imports = [
        devshell.flakeModule
      ];

      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = { config, pkgs, system, ... }:
      let
        # Add rust-overlay
        overlays = [ rust-overlay.overlays.default ];
        pkgsWithOverlays = import nixpkgs {
          inherit system overlays;
        };
        
        # Rust toolchain with WASM target (matching the original flake)
        rustToolchain = pkgsWithOverlays.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Common build inputs for Rust projects
        buildInputs = with pkgs; [
          pkg-config
          openssl
          libiconv  # Required for ring crate on macOS
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
          darwin.apple_sdk.frameworks.CoreFoundation
          darwin.libiconv  # Explicit macOS libiconv
        ];

        nativeBuildInputs = [
          rustToolchain
          pkgs.pkg-config
          pkgs.clang
          pkgs.llvmPackages.llvm
        ];

        # Common environment setup script
        env-setup-script = pkgs.writeShellScript "env-setup" ''
          # Set macOS deployment target if on Darwin
          export MACOSX_DEPLOYMENT_TARGET="10.12"
          
          # Set SOURCE_DATE_EPOCH for reproducible builds
          export SOURCE_DATE_EPOCH="1"
          
          # Ensure C compiler is available
          export CC="${pkgs.clang}/bin/clang"
          
          # Add library paths for macOS system libraries
          ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
            export LIBRARY_PATH="${pkgs.darwin.libiconv}/lib:${pkgs.libiconv}/lib:$LIBRARY_PATH"
          ''}
        '';

        # Helper script for running the service (equivalent to cargo run-service)
        run-service-script = pkgs.writeShellScriptBin "run-service" ''
          set -e
          
          source ${env-setup-script}
          
          # Default values with configurable prover host
          PROVER_HOST="''${VALENCE_PROVER_HOST:-104.171.203.127:37282}"
          RUST_LOG_DEFAULT="info,valence_coprocessor=debug,valence_coprocessor_wasm=debug"
          
          # Use provided RUST_LOG or default
          export RUST_LOG="''${RUST_LOG:-$RUST_LOG_DEFAULT}"
          
          echo "🚀 Starting Valence co-processor service..."
          echo "RUST_LOG: $RUST_LOG"
          echo "VALENCE_PROVER_SECRET: ''${VALENCE_PROVER_SECRET:-"[not set - using public service]"}"
          echo "Prover host: $PROVER_HOST"
          echo ""
          
          # Check if VALENCE_PROVER_SECRET is set
          if [ -z "''${VALENCE_PROVER_SECRET:-}" ]; then
            echo "⚠️  Warning: VALENCE_PROVER_SECRET not set. Using public prover service."
            echo "   To use dedicated prover, set: export VALENCE_PROVER_SECRET=your_secret"
            echo ""
          fi
          
          exec ${rustToolchain}/bin/cargo run \
            --package valence-coprocessor-service \
            --profile optimized \
            -- --prover "$PROVER_HOST" "$@"
        '';

        # Helper script for running with release profile (equivalent to cargo run-service-release)
        run-service-release-script = pkgs.writeShellScriptBin "run-service-release" ''
          set -e
          
          source ${env-setup-script}
          
          PROVER_HOST="''${VALENCE_PROVER_HOST:-104.171.203.127:37282}"
          export RUST_LOG="''${RUST_LOG:-info,valence_coprocessor=debug,valence_coprocessor_wasm=debug}"
          
          echo "🚀 Starting Valence co-processor service (release mode)..."
          echo "RUST_LOG: $RUST_LOG"
          echo "VALENCE_PROVER_SECRET: ''${VALENCE_PROVER_SECRET:-"[not set]"}"
          echo ""
          
          exec ${rustToolchain}/bin/cargo run \
            --package valence-coprocessor-service \
            --release \
            -- --prover "$PROVER_HOST" "$@"
        '';

        # Linting script (equivalent to cargo lint)
        lint-script = pkgs.writeShellScriptBin "cargo-lint" ''
          source ${env-setup-script}
          
          exec ${rustToolchain}/bin/cargo clippy --all --all-targets -- -D warnings "$@"
        '';

        # Install cargo-valence globally script
        install-cargo-valence-script = pkgs.writeShellScriptBin "install-cargo-valence" ''
          source ${env-setup-script}
          
          echo "Installing cargo-valence CLI tool..."
          exec ${rustToolchain}/bin/cargo install \
            --path crates/endpoint/cli \
            --locked \
            --force
        '';

      in
      {
        # Apps that can be run with `nix run`
        apps = {
          default = {
            type = "app";
            program = "${run-service-script}/bin/run-service";
          };
          
          service = {
            type = "app";
            program = "${run-service-script}/bin/run-service";
          };
          
          service-release = {
            type = "app";
            program = "${run-service-release-script}/bin/run-service-release";
          };
          
          lint = {
            type = "app";
            program = "${lint-script}/bin/cargo-lint";
          };

          install-cargo-valence = {
            type = "app";
            program = "${install-cargo-valence-script}/bin/install-cargo-valence";
          };
        };

        # Development shells
        devshells.default = {
          name = "valence-coprocessor-dev";
          
          packages = [
            rustToolchain
            pkgs.pkg-config
            pkgs.openssl
            pkgs.libiconv  # Required for ring crate on macOS
            pkgs.clang
            pkgs.llvmPackages.llvm
            pkgs.redis  # For local Redis development
            pkgs.curl   # For API testing
            pkgs.jq     # For JSON processing
            
            # Custom scripts
            run-service-script
            run-service-release-script
            lint-script
            install-cargo-valence-script
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
            pkgs.darwin.libiconv  # Explicit macOS libiconv
          ];

          commands = [
            {
              category = "service";
              name = "start-service";
              help = "Start the Valence co-processor service (equivalent to cargo run-service)";
              command = "run-service";
            }
            {
              category = "service";
              name = "start-service-release";
              help = "Start service in release mode (equivalent to cargo run-service-release)";
              command = "run-service-release";
            }
            {
              category = "development";
              name = "lint-code";
              help = "Run clippy linting (equivalent to cargo lint)";
              command = "cargo-lint";
            }
            {
              category = "cli";
              name = "install-cli";
              help = "Install cargo-valence CLI globally";
              command = "install-cargo-valence";
            }
            {
              category = "testing";
              name = "test-service";
              help = "Test if the service is running";
              command = "curl -s http://localhost:37281/api/status || echo 'Service not running'";
            }
            {
              category = "database";
              name = "redis-start";
              help = "Start Redis server for local development";
              command = "redis-server";
            }
            {
              category = "database";
              name = "redis-client";
              help = "Open Redis CLI client";
              command = "redis-cli";
            }
          ];

          bash.extra = ''
            source ${env-setup-script}
            
            echo "🚀 Valence co-processor development environment"
            echo ""
            echo "📋 Available commands (use 'menu' to see all):"
            echo "  start-service              - Start the service (VALENCE_PROVER_SECRET=secret start-service)"
            echo "  start-service-release      - Start service in release mode"
            echo "  lint-code                  - Run clippy linting"
            echo "  install-cli                - Install cargo-valence CLI tool globally"
            echo "  test-service               - Check if service is responding"
            echo ""
            echo "📖 README examples:"
            echo "  VALENCE_PROVER_SECRET=secret start-service"
            echo "  RUST_LOG=info,valence_coprocessor=debug,valence_coprocessor_wasm=debug start-service"
            echo ""
            echo "🗄️  Redis (for local development):"
            echo "  redis-start                - Start Redis server"
            echo "  redis-client               - Redis CLI client"
            echo ""
            echo "🌍 Public service: http://prover.timewave.computer:37281/"
            echo "📱 CLI installation: cargo install --git https://github.com/timewave-computer/valence-coprocessor.git --locked cargo-valence"
          '';
        };
      };
    });
} 