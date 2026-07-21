{ pkgs }:

let
  inherit (pkgs) lib;
in
pkgs.rustPlatform.buildRustPackage {
  pname = "compass";
  version = (lib.importTOML ../Cargo.toml).package.version;

  # Keep the source closure tight: the crate and its manifests only. `context/`,
  # `.github/`, `target/` and `.git/` are deliberately excluded — nothing in the
  # build reads them, and including them would rebuild the package on every doc
  # edit. `maybeMissing` keeps evaluation working while the crate is still being
  # written (Cargo.lock and build.rs may not exist yet).
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      (lib.fileset.maybeMissing ../Cargo.lock)
      (lib.fileset.maybeMissing ../build.rs)
      ../rust-toolchain.toml
      (lib.fileset.maybeMissing ../src)
      (lib.fileset.maybeMissing ../tests)
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  # No .git in the sandbox — a build script that embeds a git rev must degrade to
  # a placeholder rather than fail. Handing it an explicit marker keeps the
  # embedded version honest about where the binary came from.
  env.COMPASS_BUILD_REV = "nix";

  nativeBuildInputs = [ pkgs.installShellFiles ];

  doCheck = true;

  meta = {
    description = "Durable planning intent for coding agents";
    homepage = "https://github.com/compoundingtech/compass";
    license = lib.licenses.mit;
    mainProgram = "compass";
    platforms = lib.platforms.unix;
  };
}
