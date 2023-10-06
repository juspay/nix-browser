# Nix module for the Rust part of the project
#
# This uses https://github.com/srid/dioxus-desktop-template/blob/master/nix/flake-module.nix
{
  perSystem = { config, self', pkgs, lib, system, ... }:
    let
      rustBuildInputs = lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
        IOKit
        # For when we start using Tauri
        Carbon
        WebKit
        Security
        Cocoa
      ]);
    in
    {
      dioxus-desktop = {
        inherit rustBuildInputs;
        overrideCraneArgs = oa:
          let
            # 'cargo leptos test' doesn't run tests for all crates in the
            # workspace. We do it here.
            run-test = pkgs.writeShellApplication {
              name = "run-test";
              text = ''
                set -xe

                ${oa.cargoTestCommand or ""}

                # Disable tests on macOS for https://github.com/garnix-io/issues/issues/69
                # If/when we move to Jenkins, this won't be necessary.
                ${if !pkgs.stdenv.isDarwin
                  then ''
                    # Run `cargo test` using the same settings as `cargo leptos test`
                    # In particular: target-dir and features
                    cargo test --target-dir=target/server --no-default-features --features=ssr
                    cargo test --target-dir=target/front --no-default-features --features=hydrate
                  ''
                  else ""
                }
              '';
            };
          in
          {
            nativeBuildInputs = (oa.nativeBuildInputs or [ ]) ++ [
              pkgs.nix # cargo tests need nix
            ];
            cargoTestCommand = lib.getExe run-test;
            meta.description = "WIP: nix-browser";
          };
      };

      packages = {
        default = self'.packages.nix-browser;
        nix-health = config.dioxus-desktop-template.craneLib.buildPackage {
          inherit (config.dioxus-desktop-template) src;
          pname = "nix-health";
          nativeBuildInputs = [
            pkgs.nix # cargo tests need nix
          ];
          cargoExtraArgs = "-p nix_health --features ssr";
          # Disable tests on macOS for https://github.com/garnix-io/issues/issues/69
          # If/when we move to Jenkins, this won't be necessary.
          doCheck = !pkgs.stdenv.isDarwin;
        };
      };

      devShells.rust = pkgs.mkShell {
        inputsFrom = [
          self'.devShells.nix-browser
        ];
        packages = with pkgs; [
          cargo-watch
          cargo-expand
          cargo-nextest
          config.process-compose.cargo-doc-live.outputs.package
        ];
        shellHook = ''
          echo
          echo "🍎🍎 Run 'just <recipe>' to get started"
          just
        '';
      };
    };
}
