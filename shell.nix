let
  
  toolchainTomlPath = ./toolchain.toml;
  toolchainToml = (builtins.fromTOML (builtins.readFile toolchainTomlPath)).toolchain;

  rust_overlay =
    import (builtins.fetchGit {
      url = "https://github.com/oxalica/rust-overlay";
      rev = "a795148ffbcc77f2b592d50ceebe36147e623a77";
    });

  pinned = builtins.fetchGit {
    url = "https://github.com/nixos/nixpkgs/";
    ref = "refs/tags/23.05";
  };

  pkgs = import pinned { overlays = [ rust_overlay ]; };

  rust = pkgs.rust-bin.stable.${toolchainToml.channel}.default.override {
    extensions = toolchainToml.components;
    targets = toolchainToml.targets;
  };

in
pkgs.mkShell {
buildInputs = [
    rust
  ] ++ (with pkgs; [
    clang
    pkg-config
    bacon
  ]);

  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  PROTOC = "${pkgs.protobuf}/bin/protoc";
  ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";
}
