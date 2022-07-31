with (import <unstable> {});

mkShell rec {
  name = "casql";

  nativeBuildInputs = [ rustc cargo gcc ];
  buildInputs = [  clippy rustfmt ];

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
