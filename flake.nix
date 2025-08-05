{
  description = "Valence co-processor: Zero-Knowledge Virtual Machine service for cross-chain applications";

  nixConfig = {
    extra-experimental-features = "nix-command flakes";
    allow-import-from-derivation = true;
    extra-substituters = "https://timewave.cachix.org";
    extra-trusted-public-keys = ''
      timewave.cachix.org-1:nu3Uqsm3sikI9xFK3Mt4AD4Q6z+j6eS9+kND1vtznq4=
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
    crate2nix.url = "github:timewave-computer/crate2nix";
    sp1-nix.url = "github:timewave-computer/sp1.nix";
    fp-addons.url = "github:timewave-computer/flake-parts-addons";
    system-manager.url = "github:numtide/system-manager";
  };

  outputs = { self, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } ({lib , moduleWithSystem, ...}: {
      imports = [
        inputs.devshell.flakeModule
        inputs.crate2nix.flakeModule
        inputs.fp-addons.flakeModules.tools
      ];

      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = { pkgs, system, self', inputs', config, ... }: {
        crate2nix = {
          cargoNix = ./Cargo.nix;
          devshell.name = "default"; # adds update-cargo-nix command to devshells.default
          profile = "optimized";
          crateOverrides = inputs'.sp1-nix.tools.crateOverrides // {
            valence-coprocessor-service = attrs: {
              meta.mainProgram = "coprocessor";
            };
            valence-coprocessor-prover = attrs: {
              meta.mainProgram = "prover";
            };
          };
        };
        
        checks = with config.crate2nix.checks; {
          inherit cargo-valence;
          prover = valence-coprocessor-prover;
          service = valence-coprocessor-service;
        };

        packages = {
          prover = config.crate2nix.packages.valence-coprocessor-prover.override {
            features = [ "gpu" ];
          };
          service = config.crate2nix.packages.valence-coprocessor-service;
          inherit (config.crate2nix.packages) cargo-valence;
        };

        devshells.default = {
          name = "valence-coprocessor-dev";

          env = [
            {
              name = "CC";
              value = "${pkgs.clang}/bin/clang";
            }
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            {
              name = "LIBRARY_PATH";
              prefix = "${pkgs.darwin.libiconv}/lib:${pkgs.libiconv}/lib";
            }
            {
              name = "MACOS_DEPLOYMENT_TARGET";
              value = "10.03";
            }
          ];
          
          packages = with pkgs; [
            rustc
            pkg-config
            openssl
            libiconv  # Required for ring crate on macOS
            clang
            llvmPackages.llvm
            redis  # For local Redis development
            curl   # For API testing
            jq     # For JSON processing
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.libiconv  # Explicit macOS libiconv
          ];

          commands = [
            {
              name = "lint-code";
              help = "Run clippy linting (equivalent to cargo lint)";
              command = "${pkgs.cargo}/bin/cargo clippy --all --all-targets -- -D warnings \"$@\"";
            }
            {
              name = "test-service";
              help = "Test if the service is running";
              command = "curl -s http://localhost:37281/api/status || echo 'Service not running'";
            }
          ];

          bash.extra = ''
            echo "🗄️  Redis (for local development):"
            echo "  redis-server               - Start Redis server"
            echo "  redis-cli                  - Redis CLI client"
            echo ""
            echo "🌍 Public service: https://service.coprocessor.valence.zone/"
            echo "📱 CLI installation: cargo install --git https://github.com/timewave-computer/valence-coprocessor.git --locked cargo-valence"
          '';
        };
        devshells.coprocessor = {
          commands = [
            {
              package = self'.packages.cargo-valence;
            }
            {
              package = self'.packages.service;
            }
            {
              package = self'.packages.prover;
            }
          ];
        };
      };

      flake.systemModules.prover = moduleWithSystem (
        { self', ... }:
        { lib, pkgs, ...}:
        {
          systemd.services = {
            valence-coprocessor-prover = {
              enable = true;
              serviceConfig = {
                Type = "simple";
                User = "valence-coprocessor";
                SupplementaryGroups = [ "docker" ];
                StateDirectory = "valence-coprocessor-prover";
                ExecStart = lib.getExe self'.packages.prover;
                Restart = "on-failure";
              };
              preStop = ''
                docker kill $(docker ps | grep moongate | awk '{print $1}')
              '';
              environment = {
                HOME = "/var/lib/valence-coprocessor-prover";
              };
              # Ensure access to docker binary
              # /usr is treated as a nix backage path
              # and /usr/bin and /usr/sbin will be added to $PATH
              path = with pkgs; [ "/usr" gawk gnugrep ]; # to get access to docker
              wantedBy = [ "system-manager.target" ];
            };
          };
        }
      );

      flake.nixosModules.service = moduleWithSystem (
        { self', ... }:
        { config, lib, ... }:
        let
          cfg = config.services.valence-coprocessor.service;
        in
        {
          options = {
            services.valence-coprocessor.service = {
              package = lib.mkOption {
                type = lib.types.package;
                default = self'.packages.service;
              };
              flags = lib.mkOption {
                type = lib.types.listOf (lib.types.str);
                default = [];
              };
            };
          };
          config = {
            systemd.services = {
              valence-coprocessor = {
                enable = true;
                serviceConfig = {
                  Type = "simple";
                  DynamicUser = true;
                  StateDirectory = "valence-coprocessor";
                  ExecStart = "${lib.getExe cfg.package} ${lib.concatStringsSep " " cfg.flags}";
                };
                wantedBy = [ "multi-user.target" ];
              };
            };
          };
        }
      );
    });
} 
