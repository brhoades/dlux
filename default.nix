let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in
  with nixpkgs;
  stdenv.mkDerivation {
    name = "rust-dlux";
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [
      rustup

      pkg-config
      llvmPackages.libclang
      openssl
      libudev
    ];
  }

