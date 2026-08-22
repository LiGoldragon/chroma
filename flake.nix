{
  description = "Chroma — one Rust daemon for theme, warmth, and brightness, controlled via DOTOS.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = {
      url = "github:LiGoldragon/rust-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-build }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust = rust-build.lib.${system}.fromToolchainFile pkgs {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
        };

        inherit (rust) craneLib toolchain;
        src = rust.cleanCargoSource ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          CHROMA_TEST_SHELL = "${pkgs.bash}/bin/bash";
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        chromaPackage = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
        pythonWithDbusNext = pkgs.python3.withPackages (pythonPackages: [
          pythonPackages.dbus-next
        ]);
        fakeGammaService = pkgs.writeShellApplication {
          name = "chroma-fake-gamma-service";
          runtimeInputs = [
            pythonWithDbusNext
          ];
          text = ''
            exec python ${./scripts/chroma-fake-gamma-service.py} "$@"
          '';
        };
        fakeGhosttyService = pkgs.writeShellApplication {
          name = "chroma-fake-ghostty-service";
          runtimeInputs = [
            pythonWithDbusNext
          ];
          text = ''
            exec python ${./scripts/chroma-fake-ghostty-service.py} "$@"
          '';
        };
        chromaSandboxTerminal = pkgs.writeShellApplication {
          name = "chroma-sandbox-terminal";
          runtimeInputs = [
            chromaPackage
            fakeGammaService
            fakeGhosttyService
            pkgs.coreutils
            pkgs.dbus
            pkgs.ghostty
            pkgs.gnugrep
            pkgs.inotify-tools
            pkgs.ripgrep
            pkgs.systemd
          ];
          text = builtins.readFile ./scripts/chroma-sandbox-terminal;
        };
      in
      {
        packages.default = chromaPackage;
        packages.chroma-sandbox-terminal = chromaSandboxTerminal;

        checks.default = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });
        checks.session-dbus = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
          nativeBuildInputs = [ pkgs.dbus ];
          cargoTestCommand = "dbus-run-session -- cargo test --lib actual_theme_dbus_service_binds_the_real_protocol_to_unique_bus_owners -- --ignored";
        });
        checks.sandbox-terminal = pkgs.runCommand "chroma-sandbox-terminal-check"
          {
            nativeBuildInputs = [
              chromaSandboxTerminal
            ];
          }
          ''
            chroma-sandbox-terminal \
              --no-systemd \
              --no-terminal \
              --artifact-root "$out"
          '';

        apps.sandbox-terminal = flake-utils.lib.mkApp {
          drv = chromaSandboxTerminal;
        };
        apps.sandbox-check = flake-utils.lib.mkApp {
          drv = pkgs.writeShellApplication {
            name = "chroma-sandbox-check";
            runtimeInputs = [
              chromaSandboxTerminal
            ];
            text = ''
              exec chroma-sandbox-terminal --no-systemd --no-terminal "$@"
            '';
          };
        };

        devShells.default = pkgs.mkShell {
          name = "chroma";
          packages = [
            pkgs.jujutsu
            pkgs.pkg-config
            toolchain
          ];
        };
      }
    );
}
