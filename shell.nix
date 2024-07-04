let
  rust_overlay =
    import (builtins.fetchGit {
      url = "https://github.com/oxalica/rust-overlay";
      ref = "refs/heads/master"; # to make sure rust version is available from toolchain.toml
    });

  pinned = builtins.fetchGit {
    url = "https://github.com/nixos/nixpkgs/";
    ref = "refs/tags/24.05";
  };

  pkgs = import pinned { overlays = [ rust_overlay ]; };

  rust = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
in
pkgs.mkShell {
buildInputs = [
    rust
  ] ++ (with pkgs; [
    clang
    pkg-config
  ]);
 
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  PROTOC = "${pkgs.protobuf}/bin/protoc";
  ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";
}
