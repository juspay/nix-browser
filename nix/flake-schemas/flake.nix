{
  inputs = {
    flake-schemas.url = "github:DeterminateSystems/flake-schemas";
    systems.url = "github:nix-systems/default";
  };
  outputs = inputs@{ flake-schemas, ... }:
    let
      appsSchema = {
        version = 1;
        doc = ''
          The `apps` output provides commands available via `nix run`.
        '';
        inventory = output:
          flake-schemas.lib.mkChildren (builtins.mapAttrs
            (system: apps:
              let
                forSystems = [ system ];
              in
              {
                inherit forSystems;
                children =
                  builtins.mapAttrs
                    (appName: app: {
                      inherit forSystems;
                      evalChecks.isValidApp =
                        app ? type
                        && app.type == "app"
                        && app ? program
                        && builtins.isString app.program;
                      what = "app";
                      shortDescription = app.meta.description or "";
                    })
                    apps;
              })
            output);
      };
      nixosConfigurationsSchema = {
        version = 1;
        doc = ''
          The `nixosConfigurations` flake output defines [NixOS system configurations](https://nixos.org/manual/nixos/stable/#ch-configuration).
        '';
        inventory = output: flake-schemas.lib.mkChildren (builtins.mapAttrs
          (configName: machine:
            {
              what = "NixOS configuration";
              derivation = machine.config.system.build.toplevel;
              forSystems = [ machine.pkgs.stdenv.system ];
              # Force evaluate this derivation on all systems. See: https://github.com/juspay/omnix/pull/277#discussion_r1760164052
              evalOnAllSystems = true;
            })
          output);
      };
      homeConfigurationsSchema = {
        version = 1;
        doc = ''
          The `homeConfigurations` flake output defines [Home Manager configurations](https://github.com/nix-community/home-manager).
        '';
        inventory = output: flake-schemas.lib.mkChildren (builtins.mapAttrs
          (configName: this:
            {
              what = "Home Manager configuration";
              derivation = this.activationPackage;
              forSystems = [ this.activationPackage.system ];
              evalOnAllSystems = true;
            })
          output);
      };
      darwinConfigurationsSchema = {
        version = 1;
        doc = ''
          The `darwinConfigurations` flake output defines [nix-darwin configurations](https://github.com/LnL7/nix-darwin).
        '';
        inventory = output: flake-schemas.lib.mkChildren (builtins.mapAttrs
          (configName: this:
            {
              what = "nix-darwin configuration";
              derivation = this.system;
              forSystems = [ this.system.system ];
              evalOnAllSystems = true;
            })
          output);
      };
      processComposeSchema = {
        # Enabling flake-parts `debug` flag is required for this schema to work.
        # https://flake.parts/options/flake-parts.html#opt-debug
        # TODO: https://github.com/Platonic-Systems/process-compose-flake should provide schema for it self
        # So that omnix at runtime can fetch and merge the schema from the flake
        version = 1;
        doc = ''
          The `apps` output provides commands available via `nix run`.
        '';
        inventory = output:
          flake-schemas.lib.mkChildren (builtins.listToAttrs (map
            (system: {
              name = system;
              value = flake-schemas.lib.mkChildren (builtins.mapAttrs
                (processes: definition:
                  {
                    evalChecks.isValidProcess =
                      definition ? settings;
                    what = "Process Compose";
                    evalOnAllSystems = true;
                  })
                (output.${system}.process-compose or { }));
            })
            (import inputs.systems)));
      };

    in
    {
      schemas = flake-schemas.schemas // {
        # Until upstream is merged: https://github.com/DeterminateSystems/flake-schemas/pull/31
        apps = appsSchema;
        nixosConfigurations = nixosConfigurationsSchema;
        homeConfigurations = homeConfigurationsSchema;
        darwinConfigurations = darwinConfigurationsSchema;
        allSystems = processComposeSchema;
      };
    };
}
