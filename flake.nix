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

        # Build identity comes from the flake's own source metadata: a clean
        # checkout has `rev`, a dirty tree has `dirtyRev`, and a source tree with
        # no git at all (a tarball, a `path:` flake) has neither. The binary must
        # build in all three cases, so the last one degrades to a named unknown
        # rather than failing.
        compass = import ./nix/build.nix {
          inherit pkgs;
          rev = self.rev or self.dirtyRev or "unknown";
          commitTs = self.lastModified or 0;
          dirty = self ? dirtyRev;
        };
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
