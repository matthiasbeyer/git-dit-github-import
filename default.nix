{ pkgs ? (import <nixpkgs> {}) }:

let
  env = with pkgs.latest.rustChannels.stable; [
    rust
    cargo
  ];

  dependencies = with pkgs; [
    gcc
    openssl
    pkgconfig
    zlib
    libssh2
  ];
in

pkgs.stdenv.mkDerivation rec {
    name = "git-dit-github-import";
    src = /var/empty;
    version = "0.0.0";

    buildInputs = env ++ dependencies;

}

