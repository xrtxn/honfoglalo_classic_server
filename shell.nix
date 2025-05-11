let
  rust-overlay = import (
    builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"
  );
  pkgs = import <nixpkgs> { overlays = [ rust-overlay ]; };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    rust-bin.stable.latest.default # Latest stable Rust
  ];

  RUST_SRC_PATH = "${pkgs.rust-bin.stable.latest.rust-src}/lib/rustlib/src/rust/library";
}
