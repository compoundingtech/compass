{ pkgs }:

pkgs.mkShell {
  packages = [
    pkgs.cargo
    pkgs.rustc
    pkgs.rust-analyzer
    pkgs.rustfmt
    pkgs.clippy
  ]
  ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
    pkgs.libiconv
  ];

  env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
}
