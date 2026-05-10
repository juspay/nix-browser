# `om init` registry's canonical source
#
# Get JSON using:
# nix eval --json .#registry | jq
{
  inputs = {
    haskell-flake.url = "github:srid/haskell-flake";
    haskell-flake.flake = false;

    haskell-template.url = "github:srid/haskell-template";
    haskell-template.flake = false;

    python-nix-template.url = "github:sciexp/python-nix-template";
    python-nix-template.flake = false;

    rust-nix-template.url = "github:srid/rust-nix-template";
    rust-nix-template.flake = false;

    nixos-unified-template.url = "github:juspay/nixos-unified-template";
    nixos-unified-template.flake = false;
  };

  outputs = inputs: {
    registry =
      builtins.mapAttrs
        (k: v: v.outPath)
        (builtins.removeAttrs inputs [ "self" ]);
  };
}
