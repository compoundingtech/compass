{
  pkgs,
  # Build identity for the binary, supplied by flake.nix from the flake's own
  # source metadata. See `buildStamp` below.
  rev ? "unknown",
  commitTs ? 0,
  dirty ? false,
}:

let
  inherit (pkgs) lib;

  cargoVersion = (lib.importTOML ../Cargo.toml).package.version;

  # `build.rs` reads `CLI_BUILD_STAMP` and embeds it verbatim; `src/version.rs`
  # parses it back out for `compass version`. The Nix sandbox has no `.git` and
  # no `git`, so the build script's own git fallback cannot fire — this is the
  # only path by which a Nix-built binary learns what it was built from.
  buildStamp = builtins.toJSON {
    type = "nix";
    version = cargoVersion;
    inherit rev;
    inherit commitTs;
    inherit dirty;
  };
in
pkgs.rustPlatform.buildRustPackage {
  pname = "compass";
  version = cargoVersion;

  # Keep the source closure tight: the crate and its manifests only. `context/`,
  # `.github/`, `target/` and `.git/` are deliberately excluded — nothing in the
  # build reads them, and including them would rebuild the package on every doc
  # edit.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../build.rs
      ../rust-toolchain.toml
      ../src
      (lib.fileset.maybeMissing ../tests)
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  env.CLI_BUILD_STAMP = buildStamp;

  doCheck = true;

  meta = {
    description = "Durable planning intent for coding agents";
    homepage = "https://github.com/compoundingtech/compass";
    license = lib.licenses.mit;
    mainProgram = "compass";
    platforms = lib.platforms.unix;
  };
}
