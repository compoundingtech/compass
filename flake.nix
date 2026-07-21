{
  description = "compass - durable planning intent for coding agents";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        compass = import ./nix/build.nix { inherit pkgs; };
      in
      {
        packages = {
          default = compass;
          inherit compass;
        };

        devShells.default = import ./nix/rust-dev-shell.nix { inherit pkgs; };

        checks = {
          inherit compass;
          smoke = import ./nix/checks/smoke.nix {
            inherit pkgs;
            compassPackage = compass;
          };
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
