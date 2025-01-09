{
  description = "Development shell with rust and additional tooling";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    zombienet_overlay = {
      url = "github:paritytech/zombienet/refs/tags/v1.3.118";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, zombienet_overlay }: flake-utils.lib.eachDefaultSystem (system:
  let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default zombienet_overlay.overlays.default ];
    };
    
   nativeBuildInputs = with pkgs; [
        (rust-bin.fromRustupToolchainFile ./toolchain.toml)
        clang
        rocksdb
        llvmPackages.libclang.lib
        pkg-config
        cargo-nextest
        git-cliff
        toml-cli
        cargo-deb
        polkadot
        zombienet.default
      ];
  in {
    devShell = pkgs.mkShell {
      inherit nativeBuildInputs;
     
      PROTOC = "${pkgs.protobuf}/bin/protoc";
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeBuildInputs;

      shellHook = ''
        cargo install --list | grep zepter > /dev/null || cargo install zepter@1.5.1
        export PATH="$PATH:$HOME/.cargo/bin" 
      '';
    };
  });
}
