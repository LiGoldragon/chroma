{
  description = "Chroma — one Rust daemon for theme, warmth, and brightness, controlled via Datom.";

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
        ethosMapFilter =
          path: type:
          type == "regular" && toString path == toString ./chroma.ethos;
        src = rust.cleanSource {
          root = ./.;
          extraFilters = [ ethosMapFilter ];
        };
        commonArgs = {
          inherit src;
          strictDeps = true;
          CHROMA_TEST_SHELL = "${pkgs.bash}/bin/bash";
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        chromaPackage = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
        chromaSetDarkTheme = pkgs.writeShellApplication {
          name = "chroma-set-dark-theme";
          runtimeInputs = [ chromaPackage ];
          text = builtins.readFile ./scripts/chroma-set-dark-theme;
        };
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
            chromaSetDarkTheme
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
        packages.chroma-set-dark-theme = chromaSetDarkTheme;

        checks.default = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });
        checks.session-dbus = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
          nativeBuildInputs = [ pkgs.dbus ];
          checkPhase = ''
            runHook preCheck
            dbus-run-session --config-file ${pkgs.dbus}/share/dbus-1/session.conf -- \
              cargo test --release --locked --lib -- --ignored
            runHook postCheck
          '';
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
        checks.set-dark-theme-example = pkgs.runCommand "chroma-set-dark-theme-example-check"
          {
            nativeBuildInputs = [ chromaSandboxTerminal ];
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
        apps.set-dark-theme = flake-utils.lib.mkApp {
          drv = chromaSetDarkTheme;
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
